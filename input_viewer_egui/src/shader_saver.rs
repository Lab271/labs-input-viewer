// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! The pure fragment-shader screensavers, on the GPU: the Electron app's
//! shaders byte for byte, plus its post-FX chain (HDR scene -> bloom mips ->
//! ACES -> dither).
//!
//! The GLSL in `shaders/` is generated from the Electron sources by
//! `shaders/extract.mjs`, so a tuning change there is one regenerate away
//! rather than a hand port. Only the header differs: desktop GLSL 3.30, which is
//! what eframe's glow backend gets on macOS and Windows, instead of ES 3.00.
//! The render options below are transcribed from each saver's
//! `createShaderScreensaver(...)` call; the chain's defaults (ACES, dither,
//! exposure 1) apply wherever a saver does not set them.
//!
//! This runs inside an egui paint callback. egui restores its own GL state
//! after a callback, but it leaves the scissor test on, so every pass here
//! disables it first -- otherwise the bloom mips get clipped to the window's
//! clip rect and the bloom goes lopsided.
//!
//! The chain is shared: `render` runs a scene a saver draws itself (Weather's
//! sky and precipitation passes) through the same targets and passes.

use std::collections::HashMap;
use std::sync::Mutex;

use eframe::glow::{self, HasContext};
use crate::filelog::{log_error, log_info};

const BRIGHT: &str = include_str!("../shaders/post-bright.frag");
const DOWN: &str = include_str!("../shaders/post-downsample.frag");
const UP: &str = include_str!("../shaders/post-upsample.frag");
const COMPOSITE: &str = include_str!("../shaders/post-composite.frag");

/// Same as gl-base.js FRAGMENT_HEADER, desktop dialect.
const FRAGMENT_HEADER: &str = "#version 330 core
uniform vec3 iResolution;
uniform float iTime;
uniform int iFrame;
uniform vec4 iSeed;
out vec4 outColor;
";

const MAINIMAGE_FOOTER: &str = "
void main() {
  vec4 color = vec4(0.0);
  mainImage(color, gl_FragCoord.xy);
  outColor = color;
}";

/// One oversized triangle from gl_VertexID, so no vertex buffer is needed.
pub const VERTEX: &str = "#version 330 core
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}";

const BLOOM_MIPS: usize = 6;

/// A saver's `postFX.bloom`.
pub struct Bloom {
    pub threshold: f32,
    pub knee: f32,
    pub intensity: f32,
    pub radius: f32,
}

/// One pure fragment-shader saver.
pub struct ShaderSaver {
    pub name: &'static str,
    frag: &'static str,
    bloom: Bloom,
    exposure: f32,
    /// `antialias`: the most samples per pixel it asks for; 0 is one.
    antialias: u32,
}

const fn bloom(threshold: f32, knee: f32, intensity: f32, radius: f32) -> Bloom {
    Bloom { threshold, knee, intensity, radius }
}

/// Order is the rotation's and the `--screensaver=NAME` lookup's.
pub const SAVERS: &[ShaderSaver] = &[
    ShaderSaver { name: "Plasma", frag: include_str!("../shaders/plasma.frag"),
        bloom: bloom(0.85, 0.4, 0.5, 1.0), exposure: 1.0, antialias: 0 },
    ShaderSaver { name: "Flow Field", frag: include_str!("../shaders/flow-field.frag"),
        bloom: bloom(0.95, 0.35, 0.28, 0.9), exposure: 1.0, antialias: 0 },
    ShaderSaver { name: "Julia Family", frag: include_str!("../shaders/julia-family.frag"),
        bloom: bloom(0.17, 0.3, 0.3, 0.8), exposure: 1.0, antialias: 4 },
    ShaderSaver { name: "Burning Ship", frag: include_str!("../shaders/burning-ship.frag"),
        bloom: bloom(0.66, 0.3, 0.3, 0.8), exposure: 1.0, antialias: 4 },
    ShaderSaver { name: "Moire Interference", frag: include_str!("../shaders/moire.frag"),
        bloom: bloom(1.00, 0.35, 0.28, 0.75), exposure: 1.0, antialias: 0 },
    ShaderSaver { name: "Bicycle Horizon", frag: include_str!("../shaders/bicycle-horizon.frag"),
        bloom: bloom(1.15, 0.45, 0.45, 1.05), exposure: 0.93, antialias: 0 },
];

/// gl-base.js `resolveSampleCount`: fewer samples as the canvas grows. A
/// pixel on the wall is already small in angular terms, and 4x at 6000x1200
/// would be ~14x the fragment work of a single-sampled 1080p frame.
pub fn sample_count(w: i32, h: i32, requested: u32) -> u32 {
    let pixels = f64::from(w.max(1)) * f64::from(h.max(1));
    let hd = 1920.0 * 1080.0;
    if pixels > hd * 3.0 {
        requested.min(2)
    } else if pixels > hd * 1.4 {
        requested.min(4)
    } else {
        requested
    }
}

/// gl-base.js `makeSupersampleFooter`: `mainImage` at rotated-grid offsets,
/// averaged. Rotated because a regular grid lines up with exactly the
/// horizontal and vertical features that alias worst (#116).
pub fn supersample_footer(samples: u32) -> String {
    const TWO: &[[f32; 2]] = &[[-0.25, -0.25], [0.25, 0.25]];
    const FOUR: &[[f32; 2]] = &[[-0.375, -0.125], [0.125, -0.375], [-0.125, 0.375], [0.375, 0.125]];
    // Rook pattern: one sample per row and per column.
    const EIGHT: &[[f32; 2]] = &[
        [-0.4375, -0.1875], [-0.1875, 0.3125], [0.0625, -0.4375], [0.3125, 0.0625],
        [-0.3125, 0.1875], [-0.0625, -0.3125], [0.1875, 0.4375], [0.4375, -0.0625],
    ];
    let offsets = match samples {
        2 => TWO,
        8 => EIGHT,
        _ => FOUR,
    };
    let body: Vec<String> = offsets.iter()
        .map(|[dx, dy]| format!("  mainImage(s, gl_FragCoord.xy + vec2({dx:.4}, {dy:.4}));\n  acc += s;"))
        .collect();
    format!("\nvoid main() {{\n  vec4 acc = vec4(0.0);\n  vec4 s = vec4(0.0);\n{}\n  outColor = acc / {}.0;\n}}",
        body.join("\n"), offsets.len())
}

/// The full fragment source for a saver at a sample count.
fn scene_source(saver: &ShaderSaver, samples: u32) -> String {
    let footer = if samples > 1 { supersample_footer(samples) } else { MAINIMAGE_FOOTER.to_owned() };
    format!("{FRAGMENT_HEADER}{}{footer}", saver.frag)
}

#[derive(Clone, Copy)]
struct Target {
    fbo: glow::Framebuffer,
    tex: glow::Texture,
    w: i32,
    h: i32,
}

/// The post-FX chain and its render targets. One per GL context, shared by
/// every saver that renders through it (`render`), so the wall pays for one
/// 6000x1200 HDR scene target, not one per saver.
struct Post {
    vao: glow::VertexArray,
    bright: glow::Program,
    down: glow::Program,
    up: glow::Program,
    composite: glow::Program,
    scene: Option<Target>,
    mips: Vec<Target>,
    out: Option<Target>,
    size: (i32, i32),
}

struct Gpu {
    post: Post,
    /// Per (saver, samples), compiled on first use and kept for the life of
    /// the GL context, so a re-activation does not pay the compile again.
    scenes: HashMap<(usize, u32), glow::Program>,
}

enum State {
    Uninit,
    Ready(Gpu),
    Failed,
}

static GPU: Mutex<State> = Mutex::new(State::Uninit);
/// Savers whose shader did not compile or link on this machine.
static BROKEN: Mutex<Vec<usize>> = Mutex::new(Vec::new());

/// Whether the post chain could not be set up on this machine.
pub fn post_failed() -> bool {
    matches!(*GPU.lock().unwrap(), State::Failed)
}

/// Whether this saver cannot run, so the caller can fall back to a painter
/// saver the way registry.js falls back to the DVD logo.
pub fn failed(saver: usize) -> bool {
    post_failed() || BROKEN.lock().unwrap().contains(&saver)
}

fn with_gpu(gl: &glow::Context, f: impl FnOnce(&mut Gpu)) {
    let mut state = GPU.lock().unwrap();
    if let State::Uninit = *state {
        *state = match unsafe { Gpu::new(gl) } {
            Ok(g) => State::Ready(g),
            Err(e) => {
                log_error!("[Screensaver] GPU setup failed, falling back: {e}");
                State::Failed
            }
        };
    }
    if let State::Ready(gpu) = &mut *state {
        f(gpu);
    }
}

/// Render one frame of `saver` into `viewport` (pixels, GL origin
/// bottom-left) of the framebuffer egui is drawing to.
pub fn paint(gl: &glow::Context, saver: usize, viewport: [i32; 4], time: f32, frame: i32, seed: [f32; 4]) {
    with_gpu(gl, |gpu| unsafe {
        let [_, _, w, h] = viewport;
        let samples = sample_count(w, h, SAVERS[saver].antialias);
        let Some(prog) = gpu.scene_program(gl, saver, samples) else { return };
        let s = &SAVERS[saver];
        gpu.post.run(gl, viewport, &s.bloom, s.exposure, |gl, w, h| {
            gl.use_program(Some(prog));
            let u = |name: &str| gl.get_uniform_location(prog, name);
            gl.uniform_3_f32(u("iResolution").as_ref(), w as f32, h as f32, 1.0);
            gl.uniform_1_f32(u("iTime").as_ref(), time);
            gl.uniform_1_i32(u("iFrame").as_ref(), frame);
            gl.uniform_4_f32(u("iSeed").as_ref(), seed[0], seed[1], seed[2], seed[3]);
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            true
        });
    });
}

/// Render a saver that draws its own scene -- any number of passes -- through
/// the shared post chain. `scene` is called with the HDR scene target bound
/// at (0, 0, w, h), blending, depth and scissor off, and the fullscreen
/// triangle's VAO bound; it returns false to abandon the frame.
pub fn render(gl: &glow::Context, viewport: [i32; 4], bloom: &Bloom, exposure: f32,
    scene: impl FnOnce(&glow::Context, i32, i32) -> bool) {
    with_gpu(gl, |gpu| unsafe { gpu.post.run(gl, viewport, bloom, exposure, scene) });
}

/// Compile and link a vertex + fragment pair; the info log on failure.
pub unsafe fn compile(gl: &glow::Context, vs: &str, fs: &str) -> Result<glow::Program, String> {
    unsafe {
        let prog = gl.create_program()?;
        let mut shaders = Vec::new();
        for (kind, src) in [(glow::VERTEX_SHADER, vs), (glow::FRAGMENT_SHADER, fs)] {
            let sh = gl.create_shader(kind)?;
            gl.shader_source(sh, src);
            gl.compile_shader(sh);
            if !gl.get_shader_compile_status(sh) {
                return Err(gl.get_shader_info_log(sh));
            }
            gl.attach_shader(prog, sh);
            shaders.push(sh);
        }
        gl.link_program(prog);
        if !gl.get_program_link_status(prog) {
            return Err(gl.get_program_info_log(prog));
        }
        for sh in shaders {
            gl.detach_shader(prog, sh);
            gl.delete_shader(sh);
        }
        Ok(prog)
    }
}

unsafe fn target(gl: &glow::Context, w: i32, h: i32, hdr: bool) -> Result<Target, String> {
    unsafe {
        let tex = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        let (ifmt, ty) = if hdr { (glow::RGBA16F, glow::HALF_FLOAT) } else { (glow::RGBA8, glow::UNSIGNED_BYTE) };
        gl.tex_image_2d(glow::TEXTURE_2D, 0, ifmt as i32, w, h, 0, glow::RGBA, ty, glow::PixelUnpackData::Slice(None));
        for (p, v) in [
            (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
            (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
            (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
            (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
        ] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, p, v as i32);
        }
        let fbo = gl.create_framebuffer()?;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(tex), 0);
        let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
        if status != glow::FRAMEBUFFER_COMPLETE {
            return Err(format!("framebuffer incomplete: 0x{status:x}"));
        }
        Ok(Target { fbo, tex, w, h })
    }
}

impl Gpu {
    unsafe fn new(gl: &glow::Context) -> Result<Self, String> {
        unsafe {
            let gpu = Self {
                post: Post {
                    vao: gl.create_vertex_array()?,
                    bright: compile(gl, VERTEX, BRIGHT).map_err(|e| format!("bright: {e}"))?,
                    down: compile(gl, VERTEX, DOWN).map_err(|e| format!("downsample: {e}"))?,
                    up: compile(gl, VERTEX, UP).map_err(|e| format!("upsample: {e}"))?,
                    composite: compile(gl, VERTEX, COMPOSITE).map_err(|e| format!("composite: {e}"))?,
                    scene: None,
                    mips: Vec::new(),
                    out: None,
                    size: (0, 0),
                },
                scenes: HashMap::new(),
            };
            log_info!("[Screensaver] post-FX shaders compiled");
            Ok(gpu)
        }
    }

    /// The saver's program at this sample count, compiled on first use.
    unsafe fn scene_program(&mut self, gl: &glow::Context, saver: usize, samples: u32) -> Option<glow::Program> {
        if let Some(p) = self.scenes.get(&(saver, samples)) {
            return Some(*p);
        }
        let s = &SAVERS[saver];
        let started = std::time::Instant::now();
        match unsafe { compile(gl, VERTEX, &scene_source(s, samples)) } {
            Ok(p) => {
                log_info!("[Screensaver] {} compiled ({}x) in {} ms", s.name, samples.max(1), started.elapsed().as_millis());
                self.scenes.insert((saver, samples), p);
                Some(p)
            }
            Err(e) => {
                log_error!("[Screensaver] {} failed to compile, falling back: {e}", s.name);
                BROKEN.lock().unwrap().push(saver);
                None
            }
        }
    }
}

impl Post {
    unsafe fn free_targets(&mut self, gl: &glow::Context) {
        unsafe {
            for t in self.scene.take().into_iter().chain(self.out.take()).chain(self.mips.drain(..)) {
                gl.delete_framebuffer(t.fbo);
                gl.delete_texture(t.tex);
            }
        }
    }

    /// Match the targets to the viewport, as post-fx.js `resize()` does.
    unsafe fn resize(&mut self, gl: &glow::Context, w: i32, h: i32) -> Result<(), String> {
        unsafe {
            if self.size == (w, h) && self.scene.is_some() {
                return Ok(());
            }
            self.free_targets(gl);
            self.scene = Some(target(gl, w, h, true)?);
            // The composite lands in its own target and is then blitted into
            // egui's framebuffer: the post-FX shaders read gl_FragCoord as if
            // the canvas started at (0,0), which egui's viewport does not.
            self.out = Some(target(gl, w, h, false)?);
            let (mut mw, mut mh) = ((w >> 1).max(1), (h >> 1).max(1));
            for _ in 0..BLOOM_MIPS {
                self.mips.push(target(gl, mw, mh, true)?);
                mw = (mw >> 1).max(1);
                mh = (mh >> 1).max(1);
            }
            self.size = (w, h);
            Ok(())
        }
    }

    unsafe fn pass(&self, gl: &glow::Context, prog: glow::Program, to: Target, src: &[glow::Texture], set: impl Fn(&dyn Fn(&str) -> Option<glow::UniformLocation>)) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(to.fbo));
            gl.viewport(0, 0, to.w, to.h);
            gl.use_program(Some(prog));
            for (i, &t) in src.iter().enumerate() {
                gl.active_texture(glow::TEXTURE0 + i as u32);
                gl.bind_texture(glow::TEXTURE_2D, Some(t));
            }
            set(&|name| gl.get_uniform_location(prog, name));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
    }

    unsafe fn run(&mut self, gl: &glow::Context, vp: [i32; 4], b: &Bloom, exposure: f32,
        draw_scene: impl FnOnce(&glow::Context, i32, i32) -> bool) {
        unsafe {
            let [x, y, w, h] = vp;
            if w <= 0 || h <= 0 {
                return;
            }
            if let Err(e) = self.resize(gl, w, h) {
                log_error!("[Screensaver] {e}");
                return;
            }
            let (scene, out) = (self.scene.unwrap(), self.out.unwrap());

            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::BLEND);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::FRAMEBUFFER_SRGB);

            // Scene, in HDR.
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(scene.fbo));
            gl.viewport(0, 0, w, h);
            gl.bind_vertex_array(Some(self.vao));
            let drawn = draw_scene(gl, w, h);
            // Whatever the scene left bound, the passes below need these.
            gl.disable(glow::BLEND);
            gl.bind_vertex_array(Some(self.vao));
            if !drawn {
                gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                gl.bind_vertex_array(None);
                gl.use_program(None);
                return;
            }

            // Bloom: bright pass into mip 0, down the chain, then additively
            // back up it.
            let m0 = self.mips[0];
            self.pass(gl, self.bright, m0, &[scene.tex], |u| {
                gl.uniform_1_i32(u("uSrc").as_ref(), 0);
                gl.uniform_2_f32(u("uTexel").as_ref(), 1.0 / m0.w as f32, 1.0 / m0.h as f32);
                gl.uniform_1_f32(u("uThreshold").as_ref(), b.threshold);
                gl.uniform_1_f32(u("uKnee").as_ref(), b.knee.max(1e-4));
            });
            for i in 1..self.mips.len() {
                let (src, dst) = (self.mips[i - 1], self.mips[i]);
                self.pass(gl, self.down, dst, &[src.tex], |u| {
                    gl.uniform_1_i32(u("uSrc").as_ref(), 0);
                    gl.uniform_2_f32(u("uTexel").as_ref(), 1.0 / dst.w as f32, 1.0 / dst.h as f32);
                });
            }
            gl.enable(glow::BLEND);
            gl.blend_func(glow::ONE, glow::ONE);
            for i in (1..self.mips.len()).rev() {
                let (src, dst) = (self.mips[i], self.mips[i - 1]);
                self.pass(gl, self.up, dst, &[src.tex], |u| {
                    gl.uniform_1_i32(u("uSrc").as_ref(), 0);
                    gl.uniform_2_f32(u("uTexel").as_ref(), 1.0 / dst.w as f32, 1.0 / dst.h as f32);
                    gl.uniform_1_f32(u("uRadius").as_ref(), b.radius);
                });
            }
            gl.disable(glow::BLEND);

            // Composite: scene + bloom, exposure, ACES, gamma, dither.
            self.pass(gl, self.composite, out, &[scene.tex, m0.tex], |u| {
                gl.uniform_1_i32(u("uScene").as_ref(), 0);
                gl.uniform_1_i32(u("uBloom").as_ref(), 1);
                gl.uniform_2_f32(u("uTexel").as_ref(), 1.0 / w as f32, 1.0 / h as f32);
                gl.uniform_1_f32(u("uBloomIntensity").as_ref(), b.intensity);
                gl.uniform_1_f32(u("uExposure").as_ref(), exposure);
                gl.uniform_1_i32(u("uTonemap").as_ref(), 2);
                gl.uniform_1_i32(u("uDither").as_ref(), 1);
            });
            gl.active_texture(glow::TEXTURE0);

            // Into egui's framebuffer (the default one: egui_glow paints
            // straight to the screen; see Painter::intermediate_fbo).
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(out.fbo));
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, None);
            gl.blit_framebuffer(0, 0, w, h, x, y, x + w, y + h, glow::COLOR_BUFFER_BIT, glow::NEAREST);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.bind_vertex_array(None);
            gl.use_program(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_come_down_as_the_canvas_grows() {
        assert_eq!(sample_count(1920, 1080, 4), 4);
        assert_eq!(sample_count(2560, 1440, 8), 4);
        assert_eq!(sample_count(6000, 1200, 4), 2);
        assert_eq!(sample_count(6000, 1200, 0), 0);
    }

    #[test]
    fn the_footer_matches_gl_base() {
        // gl-base.js makeSupersampleFooter(4), character for character.
        let want = "
void main() {
  vec4 acc = vec4(0.0);
  vec4 s = vec4(0.0);
  mainImage(s, gl_FragCoord.xy + vec2(-0.3750, -0.1250));
  acc += s;
  mainImage(s, gl_FragCoord.xy + vec2(0.1250, -0.3750));
  acc += s;
  mainImage(s, gl_FragCoord.xy + vec2(-0.1250, 0.3750));
  acc += s;
  mainImage(s, gl_FragCoord.xy + vec2(0.3750, 0.1250));
  acc += s;
  outColor = acc / 4.0;
}";
        assert_eq!(supersample_footer(4), want);
        assert_eq!(supersample_footer(8).matches("mainImage").count(), 8);
    }

    #[test]
    fn every_shader_defines_main_image_and_uses_the_seed() {
        for s in SAVERS {
            assert!(s.frag.contains("void mainImage("), "{}", s.name);
            assert!(s.frag.contains("iSeed"), "{} ignores iSeed", s.name);
            assert!(!s.frag.contains("#version"), "{} has its own header", s.name);
        }
    }
}
