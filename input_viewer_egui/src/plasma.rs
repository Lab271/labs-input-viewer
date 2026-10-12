// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Plasma, on the GPU: the Electron app's shader, byte for byte, plus its
//! post-FX chain (HDR scene -> bloom mips -> ACES -> dither).
//!
//! The GLSL in `shaders/` is generated from the Electron sources by
//! `shaders/extract.mjs`, so a tuning change there is one regenerate away
//! rather than a hand port. Only the header differs: desktop GLSL 3.30, which is
//! what eframe's glow backend gets on macOS and Windows, instead of ES 3.00.
//!
//! This runs inside an egui paint callback. egui restores its own GL state
//! after a callback, but it leaves the scissor test on, so every pass here
//! disables it first -- otherwise the bloom mips get clipped to the window's
//! clip rect and the bloom goes lopsided.

use std::sync::Mutex;

use eframe::glow::{self, HasContext};
use crate::filelog::{log_error, log_info};

const PLASMA: &str = include_str!("../shaders/plasma.frag");
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
const VERTEX: &str = "#version 330 core
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}";

const BLOOM_MIPS: usize = 6;

/// plasma.js `postFX` settings.
pub struct Bloom {
    pub threshold: f32,
    pub knee: f32,
    pub intensity: f32,
    pub radius: f32,
}

pub const PLASMA_BLOOM: Bloom = Bloom { threshold: 0.85, knee: 0.4, intensity: 0.5, radius: 1.0 };

#[derive(Clone, Copy)]
struct Target {
    fbo: glow::Framebuffer,
    tex: glow::Texture,
    w: i32,
    h: i32,
}

struct Gpu {
    vao: glow::VertexArray,
    scene_prog: glow::Program,
    bright: glow::Program,
    down: glow::Program,
    up: glow::Program,
    composite: glow::Program,
    scene: Option<Target>,
    mips: Vec<Target>,
    out: Option<Target>,
    size: (i32, i32),
}

enum State {
    Uninit,
    Ready(Gpu),
    Failed,
}

/// Programs are compiled on first use and kept for the life of the GL context,
/// so a re-activation does not pay the compile again.
static GPU: Mutex<State> = Mutex::new(State::Uninit);

/// Whether a previous attempt failed, so the caller can fall back to a painter
/// saver the way registry.js falls back to the DVD logo.
pub fn failed() -> bool {
    matches!(*GPU.lock().unwrap(), State::Failed)
}

/// Render one frame into `viewport` (pixels, GL origin bottom-left) of the
/// framebuffer egui is drawing to.
pub fn paint(gl: &glow::Context, viewport: [i32; 4], time: f32, frame: i32, seed: [f32; 4]) {
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
        unsafe { gpu.draw(gl, viewport, time, frame, seed) };
    }
}

unsafe fn compile(gl: &glow::Context, vs: &str, fs: &str) -> Result<glow::Program, String> {
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
            let scene_src = format!("{FRAGMENT_HEADER}{PLASMA}{MAINIMAGE_FOOTER}");
            let gpu = Self {
                vao: gl.create_vertex_array()?,
                scene_prog: compile(gl, VERTEX, &scene_src).map_err(|e| format!("plasma: {e}"))?,
                bright: compile(gl, VERTEX, BRIGHT).map_err(|e| format!("bright: {e}"))?,
                down: compile(gl, VERTEX, DOWN).map_err(|e| format!("downsample: {e}"))?,
                up: compile(gl, VERTEX, UP).map_err(|e| format!("upsample: {e}"))?,
                composite: compile(gl, VERTEX, COMPOSITE).map_err(|e| format!("composite: {e}"))?,
                scene: None,
                mips: Vec::new(),
                out: None,
                size: (0, 0),
            };
            log_info!("[Screensaver] shaders compiled");
            Ok(gpu)
        }
    }

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

    unsafe fn draw(&mut self, gl: &glow::Context, vp: [i32; 4], time: f32, frame: i32, seed: [f32; 4]) {
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
            let b = &PLASMA_BLOOM;

            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::BLEND);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::FRAMEBUFFER_SRGB);
            gl.bind_vertex_array(Some(self.vao));

            // Scene, in HDR.
            self.pass(gl, self.scene_prog, scene, &[], |u| {
                gl.uniform_3_f32(u("iResolution").as_ref(), w as f32, h as f32, 1.0);
                gl.uniform_1_f32(u("iTime").as_ref(), time);
                gl.uniform_1_i32(u("iFrame").as_ref(), frame);
                gl.uniform_4_f32(u("iSeed").as_ref(), seed[0], seed[1], seed[2], seed[3]);
            });

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
                gl.uniform_1_f32(u("uExposure").as_ref(), 1.0);
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
