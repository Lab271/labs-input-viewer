// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! No-signal screen: the split-flap board, then a screensaver.
//!
//! The Electron app has ~30 WebGL savers. Ported so far: the six pure
//! fragment-shader ones (`shader_saver.rs`), Weather (`weather.rs`), plus
//! three painter-only ones.
//! The lifecycle is Electron's: per-activation seed (logged, so a look can be
//! reproduced), rotation every ten minutes, never repeating the last pick,
//! and the DVD logo when a saver cannot start. A saver that depends on
//! something outside itself can decline the *random* rotation
//! (`is_available`, registry.js `isAvailable`); explicit selection ignores it.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2, pos2, vec2};
use crate::filelog::log_info;

pub const ROTATE_EVERY: Duration = Duration::from_secs(10 * 60);

/// A random enabled saver, avoiding the previous pick when there is any
/// other choice. None when every saver is disabled.
pub fn pick(enabled: &[bool], avoid: Option<usize>, rng: &mut Rng) -> Option<usize> {
    let on: Vec<usize> = (0..enabled.len()).filter(|&i| enabled[i]).collect();
    let others: Vec<usize> = on.iter().copied().filter(|&i| Some(i) != avoid).collect();
    let pool = if others.is_empty() { &on } else { &others };
    (!pool.is_empty()).then(|| pool[rng.index_except(pool.len(), None)])
}

/// The next enabled saver from `current` in the direction of `delta`, for
/// the stepping keys. None when every saver is disabled.
pub fn step(enabled: &[bool], current: Option<usize>, delta: isize) -> Option<usize> {
    let n = enabled.len() as isize;
    let start = current.map_or(if delta > 0 { -1 } else { 0 }, |c| c as isize);
    (1..=n).map(|k| (start + delta.signum() * k).rem_euclid(n) as usize).find(|&i| enabled[i])
}

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

/// The shader savers first, in `shader_saver::SAVERS` order (a test holds
/// the two together), then Weather, then the painter ones.
pub const NAMES: &[&str] = &[
    "Plasma", "Flow Field", "Julia Family", "Burning Ship", "Moire Interference",
    "Weather", "Starfield Warp", "DVD Logo", "Lissajous",
];
const WEATHER: usize = 5;
const STARFIELD: usize = 6;
const DVD: usize = 7;

/// Whether a saver will take a turn in the random rotation. Only Weather can
/// decline, while there is no reading -- so a wall that is offline, or has
/// the feature off, never lands on it.
pub fn is_available(index: usize) -> bool {
    index != WEATHER || crate::weather_source::current_reading().is_some()
}

/// The rotation's pick: registry.js `pickRandomIndex` within the savers
/// ticked in Settings. Prefers an enabled saver that is available, never the
/// previous one if there is another; if no enabled saver is available,
/// availability is ignored -- a repeat or a saver without data is a smaller
/// problem than a blank wall. None only when every saver is disabled.
pub fn pick_rotation(rng: &mut Rng, avoid: Option<usize>, enabled: &[bool], available: impl Fn(usize) -> bool) -> Option<usize> {
    let both: Vec<bool> = enabled.iter().enumerate().map(|(i, &e)| e && available(i)).collect();
    pick(&both, avoid, rng).or_else(|| pick(enabled, avoid, rng))
}

pub struct Screensaver {
    pub index: usize,
    pub seed: u64,
    state: State,
}

enum State {
    /// GPU, see shader_saver.rs. `seed` is gl-base.js's iSeed: four
    /// uncorrelated randoms fixed for the activation.
    Shader { saver: usize, seed: [f32; 4], frame: i32 },
    Weather(Box<crate::weather::Weather>),
    Starfield { stars: Vec<(f32, f32, f32)>, speed: f32 },
    Dvd { pos: Vec2, vel: Vec2, hue: f32 },
    Lissajous { a: f32, b: f32, phase: f32, hue: f32 },
}

impl Screensaver {
    pub fn new(index: usize, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let state = match index % NAMES.len() {
            i if i < crate::shader_saver::SAVERS.len() =>
                State::Shader { saver: i, seed: [rng.f32(), rng.f32(), rng.f32(), rng.f32()], frame: 0 },
            WEATHER => State::Weather(Box::new(crate::weather::Weather::new(seed))),
            STARFIELD => State::Starfield {
                stars: (0..600)
                    .map(|_| (rng.range(-1.0, 1.0), rng.range(-1.0, 1.0), rng.range(0.05, 1.0)))
                    .collect(),
                speed: rng.range(0.15, 0.4),
            },
            DVD => State::Dvd {
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
        log_info!("[Screensaver] {} seed={seed}", NAMES[index % NAMES.len()]);
        Self { index: index % NAMES.len(), seed, state }
    }

    pub fn name(&self) -> &'static str {
        NAMES[self.index]
    }

    pub fn paint(&mut self, painter: &egui::Painter, rect: Rect, t: f32, dt: f32) {
        // registry.js falls back to the DVD logo when a saver cannot start;
        // same here if the GPU path did not come up.
        let failed = match self.state {
            State::Shader { saver, .. } => crate::shader_saver::failed(saver),
            State::Weather(_) => crate::weather::failed(),
            _ => false,
        };
        if failed {
            let mut rng = Rng::new(self.seed);
            self.index = DVD;
            self.state = State::Dvd { pos: vec2(rng.f32(), rng.f32()), vel: vec2(0.1, 0.15), hue: rng.f32() };
        }
        painter.rect_filled(rect, 0.0, Color32::BLACK);
        let c = rect.center();
        match &mut self.state {
            State::Shader { saver, seed, frame } => {
                let (saver, seed, f) = (*saver, *seed, *frame);
                *frame += 1;
                let cb = eframe::egui_glow::CallbackFn::new(move |info, painter| {
                    let v = info.viewport_in_pixels();
                    crate::shader_saver::paint(painter.gl(), saver, [v.left_px, v.from_bottom_px, v.width_px, v.height_px], t, f, seed);
                });
                painter.add(egui::PaintCallback { rect, callback: std::sync::Arc::new(cb) });
            }
            State::Weather(w) => w.paint(painter, rect, t, dt),
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
        // Art-Net for the painter savers (ADR 0030). They have no GL target
        // of their own, so a paint callback added after their shapes samples
        // what egui has drawn so far in this rect. In Electron every saver
        // went through gl-base and fed the observer; this keeps that true.
        // Not even the callback is added while Art-Net is off.
        if !matches!(self.state, State::Shader { .. } | State::Weather(_)) && crate::artnet::observer::wanted() {
            let cb = eframe::egui_glow::CallbackFn::new(|info, painter| {
                let v = info.viewport_in_pixels();
                crate::artnet::observer::observe(painter.gl(), painter.intermediate_fbo(),
                    [v.left_px, v.from_bottom_px, v.width_px, v.height_px], painter.intermediate_fbo());
            });
            painter.add(egui::PaintCallback { rect, callback: std::sync::Arc::new(cb) });
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
    fn shader_savers_lead_the_list_in_the_same_order() {
        let shaders: Vec<_> = crate::shader_saver::SAVERS.iter().map(|s| s.name).collect();
        assert_eq!(&NAMES[..shaders.len()], &shaders[..]);
        assert_eq!(NAMES[WEATHER], "Weather");
        assert_eq!(NAMES[STARFIELD], "Starfield Warp");
        assert_eq!(NAMES[DVD], "DVD Logo");
    }

    /// Every index a sweep of picks can produce.
    fn reachable(avoid: Option<usize>, available: impl Fn(usize) -> bool) -> std::collections::BTreeSet<usize> {
        let mut rng = Rng::new(11);
        let all = [true; NAMES.len()];
        (0..4000).filter_map(|_| pick_rotation(&mut rng, avoid, &all, &available)).collect()
    }

    #[test]
    fn the_rotation_never_picks_weather_without_a_reading() {
        let seen = reachable(None, |i| i != WEATHER);
        assert!(!seen.contains(&WEATHER));
        assert_eq!(seen.len(), NAMES.len() - 1, "every other saver is still reached");
    }

    #[test]
    fn the_rotation_picks_weather_once_a_reading_arrives() {
        assert!(reachable(None, |_| true).contains(&WEATHER));
    }

    #[test]
    fn filtering_still_never_repeats_the_previous_pick() {
        for avoid in 0..NAMES.len() {
            assert!(!reachable(Some(avoid), |i| i != WEATHER).contains(&avoid), "avoid {avoid}");
        }
    }

    #[test]
    fn a_valid_index_even_when_nothing_is_available() {
        let seen = reachable(Some(3), |_| false);
        assert!(seen.iter().all(|&i| i < NAMES.len() && i != 3));
    }

    /// The only test that touches the injected reading, so parallel tests
    /// cannot race on it.
    #[test]
    fn availability_never_brings_back_a_disabled_saver() {
        let mut rng = Rng::new(5);
        let mut enabled = [false; NAMES.len()];
        enabled[WEATHER] = true;
        enabled[DVD] = true;
        for _ in 0..500 {
            // Weather has no reading: DVD is the only choice, repeats and all.
            assert_eq!(pick_rotation(&mut rng, Some(DVD), &enabled, |i| i != WEATHER), Some(DVD));
        }
        assert_eq!(pick_rotation(&mut rng, None, &[false; NAMES.len()], |_| true), None);
    }

    #[test]
    fn weather_is_available_exactly_when_there_is_a_reading() {
        crate::weather_source::inject(None);
        assert!(!is_available(WEATHER));
        assert!(is_available(DVD));
        crate::weather_source::inject(Some(crate::weather::FALLBACK));
        assert!(is_available(WEATHER));
        crate::weather_source::inject(None);
    }

    #[test]
    fn rotation_only_picks_enabled_savers() {
        let mut r = Rng::new(3);
        let enabled = [false, true, false, true];
        for _ in 0..200 {
            assert_eq!(pick(&enabled, Some(1), &mut r), Some(3));
        }
        // The only enabled one repeats rather than leaving the wall blank.
        assert_eq!(pick(&[false, true], Some(1), &mut r), Some(1));
        assert_eq!(pick(&[false, false], None, &mut r), None);
    }

    #[test]
    fn stepping_skips_disabled_savers_and_wraps() {
        let enabled = [true, false, true, false];
        assert_eq!(step(&enabled, Some(0), 1), Some(2));
        assert_eq!(step(&enabled, Some(2), 1), Some(0));
        assert_eq!(step(&enabled, Some(0), -1), Some(2));
        assert_eq!(step(&enabled, None, 1), Some(0));
        assert_eq!(step(&[false, false], Some(0), 1), None);
    }

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
