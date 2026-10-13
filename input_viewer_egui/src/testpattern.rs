// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! A motion test pattern for `--test-pattern`: does a 4K60 feed render
//! properly, and if not, how not?
//!
//! Each element answers one question by looking at the screen:
//!
//! | Element | Shows |
//! |---|---|
//! | Lamp row, one cell lit per frame | Dropped or repeated frames: in a photo with a long exposure every cell should be lit once per second, evenly. A gap is a drop, a brighter cell a repeat. |
//! | Timecode `MM:SS:FF` and a 32-bit frame-number strip | Which frame is on screen, so two halves (or a screenshot) can be compared. |
//! | Sweeping bar, red on even frames and cyan on odd | Tearing (the bar splits into two colours along a line) and blending (it goes purple-grey). Moves exactly its own width per frame, so steady motion has no gaps and no overlap -- any judder is the renderer's. |
//! | Coloured corner marks and a 4px border | Cropping and aspect: all four corners must be visible, and unstretched. |
//! | 1px line pairs and checkerboards | Scaling quality: a 4K frame on a smaller half is downscaled, and these turn to grey (good) or to moiré (aliasing). |
//! | Grey and RGB ramps | Banding and gamma. |
//!
//! The static parts are drawn once; a frame copies them and draws only what
//! moves, which is what keeps 3840x2160 at 60 fps within one core.

use eframe::egui::Color32;

const BG: Color32 = Color32::from_rgb(16, 16, 20);
const PANEL: Color32 = Color32::from_rgb(28, 28, 34);
const DIM: Color32 = Color32::from_rgb(40, 40, 48);
const LIT: Color32 = Color32::WHITE;
const EVEN: Color32 = Color32::from_rgb(255, 40, 40);
const ODD: Color32 = Color32::from_rgb(0, 230, 230);

/// Fractions of the frame, so the layout holds at any test size.
struct Layout {
    w: usize,
    h: usize,
}

impl Layout {
    fn x(&self, f: f32) -> usize {
        ((self.w as f32 * f) as usize).min(self.w)
    }
    fn y(&self, f: f32) -> usize {
        ((self.h as f32 * f) as usize).min(self.h)
    }
    fn lamp_row(&self) -> (usize, usize, usize, usize) {
        (self.x(0.04), self.y(0.05), self.x(0.96), self.y(0.11))
    }
    fn strip(&self) -> (usize, usize, usize, usize) {
        (self.x(0.04), self.y(0.13), self.x(0.96), self.y(0.17))
    }
    fn timecode(&self) -> (usize, usize, usize, usize) {
        (self.x(0.18), self.y(0.22), self.x(0.96), self.y(0.52))
    }
    fn input_number(&self) -> (usize, usize, usize, usize) {
        (self.x(0.04), self.y(0.22), self.x(0.14), self.y(0.52))
    }
    fn sweep(&self) -> (usize, usize) {
        (self.y(0.57), self.y(0.73))
    }
}

/// The pattern as YUY2 frames, the way the cards deliver 4K, made cheaply
/// enough for two 4K60 inputs: the static background is converted once, and
/// each frame redraws and converts only the rectangles that move. Converting
/// the whole frame every time capped the source at ~33 fps on the wall's
/// Threadripper (2026-10-13), and the HUD's "shown" followed it down.
pub struct YuyvSource {
    pattern: Pattern,
    /// RGBA scratch, kept equal to the background outside the moving rects.
    scratch: Vec<Color32>,
    background_yuyv: Vec<u8>,
}

impl YuyvSource {
    pub fn new(pattern: Pattern) -> Self {
        let background_yuyv = crate::yuv::rgba_to_yuyv(bytemuck::cast_slice(&pattern.background), pattern.w, pattern.h);
        Self { scratch: pattern.background.clone(), pattern, background_yuyv }
    }

    pub fn frame(&mut self, n: u64) -> Vec<u8> {
        let p = &self.pattern;
        let w = p.w;
        let rects = p.moving_rects();
        for &(x0, y0, x1, y1) in &rects {
            for y in y0..y1 {
                self.scratch[y * w + x0..y * w + x1].copy_from_slice(&p.background[y * w + x0..y * w + x1]);
            }
        }
        p.draw_moving(&mut self.scratch, n);
        let mut out = self.background_yuyv.clone();
        for r in rects {
            crate::yuv::rgba_rect_to_yuyv(bytemuck::cast_slice(&self.scratch), w, r, &mut out);
        }
        out
    }
}

pub struct Pattern {
    w: usize,
    h: usize,
    fps: u32,
    input: usize,
    background: Vec<Color32>,
}

fn fill(px: &mut [Color32], w: usize, x0: usize, y0: usize, x1: usize, y1: usize, c: Color32) {
    let h = px.len() / w.max(1);
    let (x1, y1) = (x1.min(w), y1.min(h));
    for y in y0.min(y1)..y1 {
        px[y * w + x0.min(x1)..y * w + x1].fill(c);
    }
}

/// Seven-segment digits: no font rasteriser in a capture thread.
const SEGMENTS: [u8; 10] = [0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f];

#[allow(clippy::too_many_arguments)] // a drawing primitive: position, size, glyph, colours
fn digit(px: &mut [Color32], w: usize, x: usize, y: usize, dw: usize, dh: usize, d: u8, on: Color32, off: Color32) {
    let t = (dw / 6).max(2);
    let half = dh / 2;
    // a b c d e f g
    let rects = [
        (x + t, y, x + dw - t, y + t),
        (x + dw - t, y + t, x + dw, y + half),
        (x + dw - t, y + half, x + dw, y + dh - t),
        (x + t, y + dh - t, x + dw - t, y + dh),
        (x, y + half, x + t, y + dh - t),
        (x, y + t, x + t, y + half),
        (x + t, y + half - t / 2, x + dw - t, y + half + t - t / 2),
    ];
    for (i, (x0, y0, x1, y1)) in rects.into_iter().enumerate() {
        let lit = SEGMENTS[d as usize % 10] & (1 << i) != 0;
        fill(px, w, x0, y0, x1, y1, if lit { on } else { off });
    }
}

/// Draw `text` (digits and ':') right-aligned-free, fitted to the box.
fn seven_seg(px: &mut [Color32], w: usize, (x0, y0, x1, y1): (usize, usize, usize, usize), text: &str, on: Color32, off: Color32) {
    let n = text.chars().count().max(1) as f32;
    let dh = y1 - y0;
    let dw = ((x1 - x0) as f32 / n * 0.8).min(dh as f32 * 0.55) as usize;
    let step = ((x1 - x0) as f32 / n) as usize;
    for (i, ch) in text.chars().enumerate() {
        let x = x0 + i * step + (step - dw) / 2;
        match ch {
            ':' => {
                let s = (dw / 6).max(2);
                let cx = x + dw / 2 - s / 2;
                fill(px, w, cx, y0 + dh / 3 - s / 2, cx + s, y0 + dh / 3 + s / 2, on);
                fill(px, w, cx, y0 + 2 * dh / 3 - s / 2, cx + s, y0 + 2 * dh / 3 + s / 2, on);
            }
            c => digit(px, w, x, y0, dw, dh, c.to_digit(10).unwrap_or(0) as u8, on, off),
        }
    }
}

/// `MM:SS:FF` for frame `n` at `fps`.
pub fn timecode(n: u64, fps: u32) -> String {
    let fps = fps.max(1) as u64;
    let (ff, s) = (n % fps, n / fps);
    format!("{:02}:{:02}:{:02}", (s / 60) % 100, s % 60, ff)
}

/// Left edge of the sweeping bar on frame `n`. It advances exactly one bar
/// width per frame and crosses the frame in two seconds.
pub fn sweep_x(n: u64, w: usize, fps: u32) -> (usize, usize) {
    let steps = (fps.max(1) as usize * 2).max(1);
    let bar = (w / steps).max(1);
    ((n as usize % steps) * bar, bar)
}

/// Which lamp is lit on frame `n`: one per frame, `fps` lamps.
pub fn lamp(n: u64, fps: u32) -> usize {
    (n % fps.max(1) as u64) as usize
}

impl Pattern {
    pub fn new(w: usize, h: usize, fps: u32, input: usize) -> Self {
        let mut p = Self { w, h, fps, input, background: vec![BG; w * h] };
        p.draw_static();
        p
    }

    fn draw_static(&mut self) {
        let (w, h) = (self.w, self.h);
        let l = Layout { w, h };
        let px = &mut self.background;

        // Resolution blocks: vertical 1px lines, horizontal 1px lines, 1px
        // and 2px checkerboards.
        let (by0, by1) = (l.y(0.77), l.y(0.95));
        for (i, kind) in [0, 1, 2, 3].into_iter().enumerate() {
            let bx0 = l.x(0.04 + i as f32 * 0.115);
            let bx1 = l.x(0.04 + i as f32 * 0.115 + 0.1);
            for y in by0..by1 {
                for x in bx0..bx1 {
                    let on = match kind {
                        0 => x % 2 == 0,
                        1 => y % 2 == 0,
                        2 => (x + y) % 2 == 0,
                        _ => (x / 2 + y / 2) % 2 == 0,
                    };
                    px[y * w + x] = if on { LIT } else { Color32::BLACK };
                }
            }
        }

        // Ramps: grey, then R, G, B, stacked.
        let (rx0, rx1) = (l.x(0.52), l.x(0.96));
        let rows = [(1, 1, 1), (1, 0, 0), (0, 1, 0), (0, 0, 1)];
        let band = (by1 - by0) / rows.len();
        for (i, (r, g, b)) in rows.into_iter().enumerate() {
            for x in rx0..rx1 {
                let v = ((x - rx0) * 255 / (rx1 - rx0 - 1).max(1)) as u8;
                let c = Color32::from_rgb(v * r, v * g, v * b);
                for y in by0 + i * band..by0 + (i + 1) * band {
                    px[y * w + x] = c;
                }
            }
        }

        // Panels behind the moving parts, and the unlit lamps.
        let (lx0, ly0, lx1, ly1) = l.lamp_row();
        fill(px, w, lx0, ly0, lx1, ly1, PANEL);
        let cells = self.fps.max(1) as usize;
        let cw = (lx1 - lx0) / cells;
        for i in 0..cells {
            let x = lx0 + i * cw;
            fill(px, w, x + 2, ly0 + 2, x + cw.saturating_sub(2), ly1 - 2, DIM);
        }
        let (sx0, sy0, sx1, sy1) = l.strip();
        fill(px, w, sx0, sy0, sx1, sy1, PANEL);
        seven_seg(px, w, l.input_number(), &self.input.to_string(), Color32::from_rgb(245, 190, 60), PANEL);

        // Border and corner marks: TL red, TR green, BL blue, BR yellow.
        let b = 4.min(w / 2).min(h / 2);
        fill(px, w, 0, 0, w, b, LIT);
        fill(px, w, 0, h - b, w, h, LIT);
        fill(px, w, 0, 0, b, h, LIT);
        fill(px, w, w - b, 0, w, h, LIT);
        let m = (h / 12).max(4);
        let t = (m / 5).max(2);
        for (cx, cy, c) in [
            (b, b, Color32::RED),
            (w - b - m, b, Color32::GREEN),
            (b, h - b - m, Color32::BLUE),
            (w - b - m, h - b - m, Color32::YELLOW),
        ] {
            fill(px, w, cx, cy, cx + m, cy + m, c);
            fill(px, w, cx + t, cy + t, cx + m - t, cy + m - t, BG);
        }
    }

    #[cfg(test)]
    pub fn size(&self) -> (usize, usize) {
        (self.w, self.h)
    }

    /// Frame `n` in RGBA; the reference `YuyvSource` is tested against.
    #[cfg(test)]
    pub fn frame(&self, n: u64) -> Vec<Color32> {
        let mut px = self.background.clone();
        self.draw_moving(&mut px, n);
        px
    }

    /// Everything that changes between frames lies inside these, the sweep
    /// as its whole band (the bar's last position must be cleared too).
    fn moving_rects(&self) -> [(usize, usize, usize, usize); 4] {
        let l = Layout { w: self.w, h: self.h };
        let (y0, y1) = l.sweep();
        [l.lamp_row(), l.strip(), l.timecode(), (0, y0, self.w, y1)]
    }

    fn draw_moving(&self, px: &mut [Color32], n: u64) {
        let (w, h) = (self.w, self.h);
        let l = Layout { w, h };
        let px = &mut *px;

        let (lx0, ly0, lx1, ly1) = l.lamp_row();
        let cw = (lx1 - lx0) / self.fps.max(1) as usize;
        let x = lx0 + lamp(n, self.fps) * cw;
        fill(px, w, x + 2, ly0 + 2, x + cw.saturating_sub(2), ly1 - 2, LIT);

        let (sx0, sy0, sx1, sy1) = l.strip();
        let bw = (sx1 - sx0) / 32;
        for bit in 0..32 {
            let on = (n >> (31 - bit)) & 1 == 1;
            let x = sx0 + bit * bw;
            fill(px, w, x + 2, sy0 + 2, x + bw - 2, sy1 - 2, if on { LIT } else { DIM });
        }

        seven_seg(px, w, l.timecode(), &timecode(n, self.fps), LIT, PANEL);

        let (y0, y1) = l.sweep();
        let (bx, bar) = sweep_x(n, w, self.fps);
        fill(px, w, bx, y0, bx + bar, y1, if n.is_multiple_of(2) { EVEN } else { ODD });
    }

    /// Read the frame number back from the strip.
    #[cfg(test)]
    pub fn decode_frame_number(&self, px: &[Color32]) -> u64 {
        let l = Layout { w: self.w, h: self.h };
        let (sx0, sy0, sx1, sy1) = l.strip();
        let bw = (sx1 - sx0) / 32;
        let y = (sy0 + sy1) / 2;
        (0..32).fold(0u64, |acc, bit| {
            let c = px[y * self.w + sx0 + bit * bw + bw / 2];
            (acc << 1) | (c == LIT) as u64
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_yuyv_source_matches_converting_the_whole_frame() {
        let pattern = Pattern::new(320, 180, 60, 1);
        let reference = Pattern::new(320, 180, 60, 1);
        let mut source = YuyvSource::new(pattern);
        for n in [0, 1, 2, 37, 59, 60, 61, 500] {
            let whole = crate::yuv::rgba_to_yuyv(bytemuck::cast_slice(&reference.frame(n)), 320, 180);
            assert!(source.frame(n) == whole, "frame {n} differs");
        }
    }

    #[test]
    #[ignore = "timing, run with --release --ignored --nocapture"]
    fn time_a_4k_frame() {
        let (w, h) = (3840, 2160);
        let reference = Pattern::new(w, h, 60, 1);
        let mut source = YuyvSource::new(Pattern::new(w, h, 60, 1));
        let t = std::time::Instant::now();
        for n in 0..60 {
            std::hint::black_box(crate::yuv::rgba_to_yuyv(bytemuck::cast_slice(&reference.frame(n)), w, h));
        }
        let old = t.elapsed() / 60;
        let t = std::time::Instant::now();
        for n in 0..60 {
            std::hint::black_box(source.frame(n));
        }
        println!("4K frame: whole-frame conversion {old:?}, moving rects only {:?}", t.elapsed() / 60);
    }

    #[test]
    fn timecode_rolls_over_at_the_frame_rate() {
        assert_eq!(timecode(0, 60), "00:00:00");
        assert_eq!(timecode(59, 60), "00:00:59");
        assert_eq!(timecode(60, 60), "00:01:00");
        assert_eq!(timecode(60 * 61 + 7, 60), "01:01:07");
    }

    #[test]
    fn the_sweep_moves_exactly_its_own_width_per_frame() {
        let (x0, bar) = sweep_x(10, 3840, 60);
        let (x1, _) = sweep_x(11, 3840, 60);
        assert_eq!(bar, 32);
        assert_eq!(x1 - x0, bar);
        assert_eq!(sweep_x(120, 3840, 60).0, 0, "wraps every two seconds");
    }

    #[test]
    fn one_lamp_per_frame() {
        let lit: Vec<_> = (0..120).map(|n| lamp(n, 60)).collect();
        assert_eq!(&lit[..3], &[0, 1, 2]);
        assert_eq!(lit[60], 0);
    }

    #[test]
    fn frame_number_round_trips_through_the_strip() {
        let p = Pattern::new(640, 360, 60, 1);
        for n in [0, 1, 59, 60, 12_345, u32::MAX as u64] {
            assert_eq!(p.decode_frame_number(&p.frame(n)), n);
        }
    }

    #[test]
    fn consecutive_frames_differ_and_the_corners_are_marked() {
        let p = Pattern::new(640, 360, 60, 2);
        let (a, b) = (p.frame(0), p.frame(1));
        assert_ne!(a, b);
        let (w, h) = p.size();
        let m = h / 12;
        // Just inside the 4px border, the top-left mark is red.
        assert_eq!(a[(4 + m / 10) * w + 4 + m / 10], Color32::RED);
        assert_eq!(a[(h - 5 - m / 10) * w + w - 5 - m / 10], Color32::YELLOW);
    }
}
