// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! YUY2 to RGB on the GPU.
//!
//! The wall's Elgato cards deliver 3840x2160 only as YUY2. Converted on one
//! CPU core that took ~49 ms a frame on the wall's Threadripper, capping
//! capture at ~12 fps (measured 2026-10-12). Chromium never pays that: it
//! hands the GPU the card's own bytes. So does this.
//!
//! Each frame: the packed bytes are uploaded as a half-width RGBA8 texture --
//! one texel per pixel pair, `Y0 U Y1 V` -- and a fullscreen pass writes the
//! full-size RGBA8 texture egui draws, then its mipmaps (trilinear, as for
//! every video texture: ADR 0018). Half the bytes cross the bus compared with
//! uploading RGBA, and the CPU does nothing per pixel.
//!
//! The textures are per device and registered with egui once, so a reopen or
//! a new frame size reuses them rather than leaking registrations.
//!
//! Runs from `App::ui`, where eframe's GL context is current and egui has not
//! yet painted; the state touched here is put back to what egui expects.

use eframe::egui;
use eframe::glow::{self, HasContext};

const VERTEX: &str = "#version 330 core
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}";

/// BT.709 limited range. `yuv::ycbcr_to_rgb` is the same formula on the CPU,
/// where it has unit tests.
const FRAGMENT: &str = "#version 330 core
uniform sampler2D uPacked;
out vec4 outColor;
void main() {
  ivec2 p = ivec2(gl_FragCoord.xy);
  vec4 t = texelFetch(uPacked, ivec2(p.x >> 1, p.y), 0);
  float y = ((p.x & 1) == 0) ? t.r : t.b;
  float Y = (y * 255.0 - 16.0) / 219.0;
  float cb = (t.g * 255.0 - 128.0) / 224.0;
  float cr = (t.a * 255.0 - 128.0) / 224.0;
  vec3 rgb = vec3(Y + 1.5748 * cr, Y - 0.1873 * cb - 0.4681 * cr, Y + 1.8556 * cb);
  outColor = vec4(clamp(rgb, 0.0, 1.0), 1.0);
}";

/// The conversion program, shared by every device.
pub struct Converter {
    program: glow::Program,
    vao: glow::VertexArray,
    fbo: glow::Framebuffer,
    u_packed: Option<glow::UniformLocation>,
}

impl Converter {
    /// # Safety
    /// `gl` must be the current context.
    pub unsafe fn new(gl: &glow::Context) -> Result<Self, String> {
        unsafe {
            let program = gl.create_program()?;
            let mut shaders = Vec::new();
            for (kind, src) in [(glow::VERTEX_SHADER, VERTEX), (glow::FRAGMENT_SHADER, FRAGMENT)] {
                let sh = gl.create_shader(kind)?;
                gl.shader_source(sh, src);
                gl.compile_shader(sh);
                if !gl.get_shader_compile_status(sh) {
                    return Err(gl.get_shader_info_log(sh));
                }
                gl.attach_shader(program, sh);
                shaders.push(sh);
            }
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                return Err(gl.get_program_info_log(program));
            }
            for sh in shaders {
                gl.detach_shader(program, sh);
                gl.delete_shader(sh);
            }
            Ok(Self {
                u_packed: gl.get_uniform_location(program, "uPacked"),
                program,
                vao: gl.create_vertex_array()?,
                fbo: gl.create_framebuffer()?,
            })
        }
    }
}

/// One device's textures: the packed upload and the converted picture.
pub struct GpuTexture {
    packed: glow::Texture,
    rgba: glow::Texture,
    /// What egui paints with.
    pub id: egui::TextureId,
    pub size: [usize; 2],
}

impl GpuTexture {
    /// # Safety
    /// `gl` must be the current context.
    pub unsafe fn new(gl: &glow::Context, frame: &mut eframe::Frame) -> Result<Self, String> {
        unsafe {
            let packed = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(packed));
            // texelFetch reads exact texels; no filtering on the packed data.
            for (p, v) in [
                (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
                (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
                (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, p, v as i32);
            }
            let rgba = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(rgba));
            for (p, v) in [
                (glow::TEXTURE_MIN_FILTER, glow::LINEAR_MIPMAP_LINEAR),
                (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
                (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, p, v as i32);
            }
            gl.bind_texture(glow::TEXTURE_2D, None);
            let id = frame.register_native_glow_texture(rgba);
            Ok(Self { packed, rgba, id, size: [0, 0] })
        }
    }

    unsafe fn allocate(&mut self, gl: &glow::Context, w: usize, h: usize) {
        unsafe {
            let (w, h) = (w as i32, h as i32);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.packed));
            gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, w / 2, h, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(None));
            gl.bind_texture(glow::TEXTURE_2D, Some(self.rgba));
            gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGBA8 as i32, w, h, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(None));
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
    }

    /// Upload one YUY2 frame and convert it. Width must be even.
    ///
    /// # Safety
    /// `gl` must be the current context.
    pub unsafe fn update(&mut self, gl: &glow::Context, conv: &Converter, yuyv: &[u8], w: usize, h: usize) {
        unsafe {
            if self.size != [w, h] {
                self.allocate(gl, w, h);
                self.size = [w, h];
            }
            let (wi, hi) = (w as i32, h as i32);

            // Upload. Rows are w*2 bytes, a multiple of 4, so the default
            // alignment is right; reset anyway in case egui left it at 1.
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4);
            gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.packed));
            gl.tex_sub_image_2d(glow::TEXTURE_2D, 0, 0, 0, wi / 2, hi, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(Some(&yuyv[..w * h * 2])));

            // Convert into the picture egui draws. Row y of the output comes
            // from row y of the input, so the orientation is unchanged.
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(conv.fbo));
            gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::COLOR_ATTACHMENT0, glow::TEXTURE_2D, Some(self.rgba), 0);
            gl.viewport(0, 0, wi, hi);
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::BLEND);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::FRAMEBUFFER_SRGB);
            gl.use_program(Some(conv.program));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.packed));
            gl.uniform_1_i32(conv.u_packed.as_ref(), 0);
            gl.bind_vertex_array(Some(conv.vao));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);

            // Mipmaps: video is nearly always drawn smaller than it arrives.
            gl.bind_texture(glow::TEXTURE_2D, Some(self.rgba));
            gl.generate_mipmap(glow::TEXTURE_2D);

            // Back to what egui expects to find.
            gl.bind_vertex_array(None);
            gl.use_program(None);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
    }
}
