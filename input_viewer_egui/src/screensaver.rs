// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! No-signal screen: the split-flap board, then a screensaver.
//!
//! The Electron app has ~30 WebGL savers. These are three painter-only ones,
//! enough to show the lifecycle: per-activation seed (logged, so a look can be
//! reproduced), rotation every ten minutes, and never repeating the last pick.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2, pos2, vec2};

pub const ROTATE_EVERY: Duration = Duration::from_secs(10 * 60);

/// xorshift64*; small, seedable, good enough for visuals.
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    pub fn clock_seed() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    /// Uniform in [0, 1).
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }
    /// An index in `0..n` that is not `avoid` (when n > 1).
    pub fn index_except(&mut self, n: usize, avoid: Option<usize>) -> usize {
        if n <= 1 {
            return 0;
        }
        match avoid {
            Some(a) if a < n => {
                let i = (self.next_u64() % (n as u64 - 1)) as usize;
                if i >= a { i + 1 } else { i }
            }
            _ => (self.next_u64() % n as u64) as usize,
        }
    }
}

pub const NAMES: &[&str] = &["Plasma", "Starfield", "DVD logo", "Lissajous"];

pub struct Screensaver {
    pub index: usize,
    pub seed: u64,
    state: State,
}

enum State {
    /// GPU, see plasma.rs. `seed` is gl-base.js's iSeed: four uncorrelated
    /// randoms fixed for the activation.
    Plasma { seed: [f32; 4], frame: i32 },
    Starfield { stars: Vec<(f32, f32, f32)>, speed: f32 },
    Dvd { pos: Vec2, vel: Vec2, hue: f32 },
    Lissajous { a: f32, b: f32, phase: f32, hue: f32 },
}

impl Screensaver {
    pub fn new(index: usize, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let state = match index % NAMES.len() {
            0 => State::Plasma { seed: [rng.f32(), rng.f32(), rng.f32(), rng.f32()], frame: 0 },
            1 => State::Starfield {
                stars: (0..600)
                    .map(|_| (rng.range(-1.0, 1.0), rng.range(-1.0, 1.0), rng.range(0.05, 1.0)))
                    .collect(),
                speed: rng.range(0.15, 0.4),
            },
            2 => State::Dvd {
                pos: vec2(rng.f32(), rng.f32()),
                vel: vec2(rng.range(0.08, 0.14), rng.range(0.1, 0.2)),
                hue: rng.f32(),
            },
            _ => State::Lissajous {
                a: [1.0, 2.0, 3.0, 5.0][(rng.next_u64() % 4) as usize],
                b: [2.0, 3.0, 4.0, 7.0][(rng.next_u64() % 4) as usize],
                phase: rng.f32() * std::f32::consts::TAU,
                hue: rng.f32(),
            },
        };
        eprintln!("[screensaver] {} seed={seed}", NAMES[index % NAMES.len()]);
        Self { index: index % NAMES.len(), seed, state }
    }

    pub fn name(&self) -> &'static str {
        NAMES[self.index]
    }

    pub fn paint(&mut self, painter: &egui::Painter, rect: Rect, t: f32, dt: f32) {
        // registry.js falls back to the DVD logo when a saver cannot start;
        // same here if the GPU path did not come up.
        if matches!(self.state, State::Plasma { .. }) && crate::plasma::failed() {
            let mut rng = Rng::new(self.seed);
            self.index = 2;
            self.state = State::Dvd { pos: vec2(rng.f32(), rng.f32()), vel: vec2(0.1, 0.15), hue: rng.f32() };
        }
        painter.rect_filled(rect, 0.0, Color32::BLACK);
        let c = rect.center();
        match &mut self.state {
            State::Plasma { seed, frame } => {
                let (seed, f) = (*seed, *frame);
                *frame += 1;
                let cb = eframe::egui_glow::CallbackFn::new(move |info, painter| {
                    let v = info.viewport_in_pixels();
                    crate::plasma::paint(painter.gl(), [v.left_px, v.from_bottom_px, v.width_px, v.height_px], t, f, seed);
                });
                painter.add(egui::PaintCallback { rect, callback: std::sync::Arc::new(cb) });
            }
            State::Starfield { stars, speed } => {
                let scale = rect.width().max(rect.height()) * 0.5;
                for s in stars.iter_mut() {
                    s.2 -= *speed * dt;
                    if s.2 <= 0.02 {
                        s.2 += 1.0;
                    }
                    let p = pos2(c.x + s.0 / s.2 * scale * 0.5, c.y + s.1 / s.2 * scale * 0.5);
                    if rect.contains(p) {
                        let b = ((1.0 - s.2) * 255.0) as u8;
                        painter.circle_filled(p, (1.0 - s.2) * 2.5 + 0.5, Color32::from_gray(b));
                    }
                }
            }
            State::Dvd { pos, vel, hue } => {
                let size = vec2(rect.height() * 0.5, rect.height() * 0.22);
                let room = rect.size() - size;
                *pos += *vel * dt;
                for (p, v) in [(&mut pos.x, &mut vel.x), (&mut pos.y, &mut vel.y)] {
                    if *p < 0.0 || *p > 1.0 {
                        *p = p.clamp(0.0, 1.0);
                        *v = -*v;
                        *hue = (*hue + 0.17).fract();
                    }
                }
                let min = rect.min + vec2(pos.x * room.x, pos.y * room.y);
                let box_ = Rect::from_min_size(min, size);
                let col = hsv(*hue, 0.8, 1.0);
                painter.rect_stroke(box_, 12.0, Stroke::new(4.0, col), StrokeKind::Inside);
                painter.text(box_.center(), Align2::CENTER_CENTER, "INPUT VIEWER",
                    FontId::proportional(size.y * 0.35), col);
            }
            State::Lissajous { a, b, phase, hue } => {
                let r = vec2(rect.width() * 0.45, rect.height() * 0.42);
                let n = 600;
                let pts: Vec<Pos2> = (0..=n)
                    .map(|i| {
                        let s = i as f32 / n as f32 * std::f32::consts::TAU;
                        pos2(c.x + r.x * (*a * s + *phase + t * 0.2).sin(), c.y + r.y * (*b * s).sin())
                    })
                    .collect();
                painter.add(egui::Shape::line(pts, Stroke::new(2.0, hsv((*hue + t * 0.01).fract(), 0.6, 0.9))));
            }
        }
    }
}

pub fn hsv(h: f32, s: f32, v: f32) -> Color32 {
    egui::ecolor::Hsva::new(h, s, v, 1.0).into()
}

const FLAP_CHARS: &[u8] = b" ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-";

/// The split-flap board. Each cell flips through the alphabet until it lands
/// on its letter; `age` is seconds since the board appeared.
pub fn paint_split_flap(painter: &egui::Painter, rect: Rect, lines: &[&str], age: f32) {
    painter.rect_filled(rect, 0.0, Color32::from_rgb(8, 8, 10));
    let cols = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1).max(10);
    let cell_w = (rect.width() * 0.8 / cols as f32).min(rect.height() * 0.22 / 1.4);
    let cell = vec2(cell_w, cell_w * 1.4);
    let gap = cell_w * 0.08;
    let total_h = lines.len() as f32 * (cell.y + gap * 3.0);
    let mut y = rect.center().y - total_h / 2.0;
    for (row, line) in lines.iter().enumerate() {
        let chars: Vec<char> = line.to_uppercase().chars().collect();
        let row_w = cols as f32 * (cell.x + gap);
        let x0 = rect.center().x - row_w / 2.0;
        for col in 0..cols {
            let target = chars.get(col).copied().unwrap_or(' ');
            let target_i = FLAP_CHARS.iter().position(|&c| c as char == target).unwrap_or(0);
            // Each cell settles a little after its left neighbour.
            let flips = ((age - (col + row * 3) as f32 * 0.04) * 24.0).max(0.0) as usize;
            let shown = if flips >= target_i { target } else { FLAP_CHARS[flips] as char };
            let min = pos2(x0 + col as f32 * (cell.x + gap), y);
            let r = Rect::from_min_size(min, cell);
            painter.rect_filled(r, cell.x * 0.08, Color32::from_rgb(28, 28, 32));
            painter.hline(r.x_range(), r.center().y, Stroke::new(1.5, Color32::from_rgb(8, 8, 10)));
            painter.text(r.center(), Align2::CENTER_CENTER, shown, FontId::monospace(cell.y * 0.7),
                if row == 0 { Color32::from_rgb(240, 240, 235) } else { Color32::from_rgb(245, 190, 60) });
        }
        y += cell.y + gap * 3.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let (mut a, mut b) = (Rng::new(42), Rng::new(42));
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn floats_stay_in_unit_interval() {
        let mut r = Rng::new(7);
        assert!((0..10_000).map(|_| r.f32()).all(|v| (0.0..1.0).contains(&v)));
    }

    #[test]
    fn rotation_never_repeats_the_previous_pick() {
        let mut r = Rng::new(9);
        for prev in 0..NAMES.len() {
            for _ in 0..200 {
                assert_ne!(r.index_except(NAMES.len(), Some(prev)), prev);
            }
        }
        assert_eq!(r.index_except(1, Some(0)), 0);
    }
}
