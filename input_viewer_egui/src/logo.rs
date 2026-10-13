// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! The centre logo, drawn sharp.
//!
//! The PNG is 1014x275 and the divider is ~60 px wide, so the logo is drawn
//! about 19x smaller than it is stored. Plain linear filtering at that ratio
//! skips most source pixels: the text came out jagged and shimmered as the
//! centre gap moved. Mipmaps fix the shimmer but blur. So the logo is
//! resampled on the CPU (Lanczos) to exactly the physical pixels it covers,
//! and drawn on whole pixels. A resample takes well under a millisecond and
//! happens only when the size changes -- dragging the gap slider, say.
//!
//! While the layout slides the size changes every frame, so the mipmapped
//! texture stands in rather than resampling 20 times in 320 ms. The Settings
//! diagram, which draws it tiny, uses that one too.

use eframe::egui::{self, Color32, Rect, TextureHandle, TextureOptions, Vec2, pos2, vec2};
use image::{RgbaImage, imageops};

const TRILINEAR: TextureOptions = TextureOptions::LINEAR.with_mipmap_mode(Some(egui::TextureFilter::Linear));

pub struct Logo {
    source: RgbaImage,
    /// Any size, slightly soft; for transitions and the diagram.
    pub mipmapped: TextureHandle,
    /// Resampled for one exact pixel size.
    sharp: Option<([usize; 2], TextureHandle)>,
}

impl Logo {
    pub fn load(ctx: &egui::Context) -> Option<Self> {
        let mut source = image::load_from_memory(include_bytes!("../assets/logo.png")).ok()?.to_rgba8();
        premultiply(&mut source);
        let mipmapped = ctx.load_texture("logo", to_color_image(&source), TRILINEAR);
        Some(Self { source, mipmapped, sharp: None })
    }

    pub fn size(&self) -> Vec2 {
        vec2(self.source.width() as f32, self.source.height() as f32)
    }

    /// Draw into `rect`. `settled` is false while the layout is animating.
    pub fn paint(&mut self, painter: &egui::Painter, rect: Rect, tint: Color32, settled: bool) {
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        if !settled {
            painter.image(self.mipmapped.id(), rect, uv, tint);
            return;
        }
        let ppp = painter.ctx().pixels_per_point();
        let (min, px) = snap(rect, ppp);
        if px[0] == 0 || px[1] == 0 {
            return;
        }
        if self.sharp.as_ref().is_none_or(|(size, _)| *size != px) {
            let small = imageops::resize(&self.source, px[0] as u32, px[1] as u32, imageops::FilterType::Lanczos3);
            let tex = painter.ctx().load_texture("logo-sharp", to_color_image(&small), TextureOptions::NEAREST);
            self.sharp = Some((px, tex));
        }
        let (_, tex) = self.sharp.as_ref().expect("just set");
        let r = Rect::from_min_size(min, vec2(px[0] as f32, px[1] as f32) / ppp);
        painter.image(tex.id(), r, uv, tint);
    }
}

/// The rect's top-left on a whole physical pixel, and its size in physical
/// pixels, so each texel lands on exactly one screen pixel.
fn snap(rect: Rect, ppp: f32) -> (egui::Pos2, [usize; 2]) {
    let min = (rect.min.to_vec2() * ppp).round() / ppp;
    let px = (rect.size() * ppp).round();
    (min.to_pos2(), [px.x.max(0.0) as usize, px.y.max(0.0) as usize])
}

/// Resampling straight alpha mixes the colour of fully transparent pixels
/// (black, in this PNG) into every edge, and white text on transparent came
/// out grey. Premultiplied, a transparent pixel contributes nothing.
fn premultiply(img: &mut RgbaImage) {
    for p in img.pixels_mut() {
        let a = u16::from(p[3]);
        for c in &mut p.0[..3] {
            *c = ((u16::from(*c) * a + 127) / 255) as u8;
        }
    }
}

fn to_color_image(img: &RgbaImage) -> egui::ColorImage {
    egui::ColorImage::from_rgba_premultiplied([img.width() as usize, img.height() as usize], img.as_raw())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snaps_to_whole_physical_pixels() {
        let r = Rect::from_min_size(pos2(10.3, 20.6), vec2(54.4, 14.7));
        let (min, px) = snap(r, 2.0);
        assert_eq!((min, px), (pos2(10.5, 20.5), [109, 29]));
    }
}
