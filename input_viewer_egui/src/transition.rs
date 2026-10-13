// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Transitions (ADR 0037): an input switch dips to black, a layout change
//! slides the halves. Both are interruptible and instant under reduced motion.
//!
//! Interruptible by construction: neither is a timeline with a start and an
//! end. The layout is a position between single (0) and dual (1) that moves
//! toward the target each frame, so reversing mid-way turns around where it
//! is. A dip is one per side and remembers only the picture it is leaving, so
//! pressing 2 then 3 quickly leaves input 3 visible -- the target is simply
//! whatever the side holds now.
//!
//! Electron dipped only on the picker path, not on the number keys, with no
//! rationale recorded. Here every switch dips: one rule, and a key press on
//! the wall is the switch the audience sees most.
//!
//! Freeze stays instant.

use eframe::egui::{Rect, lerp, pos2};

/// `LAYOUT_ANIM_MS` in Electron.
pub const LAYOUT_SECS: f32 = 0.32;
/// The fade to black before an input switch.
pub const DIP_SECS: f32 = 0.13;
/// How long a dip holds black waiting for the new input's first frame.
/// Past this it snaps in anyway: the half shows whatever the input has,
/// which is how a dead input becomes visible rather than a black that waits.
pub const HOLD_MAX_SECS: f32 = 2.0;

/// Move `v` toward `target` at a rate that covers 0..1 in `secs`.
pub fn approach(v: f32, target: f32, dt: f32, secs: f32, reduced: bool) -> f32 {
    if reduced || secs <= 0.0 {
        return target;
    }
    let step = dt / secs;
    if v < target { (v + step).min(target) } else { (v - step).max(target) }
}

/// Smoothstep, so the halves accelerate out and settle in.
pub fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Left half, divider and right half at `t` between single (0) and dual (1).
/// At 1 this is `dual`; at 0 the left half is the whole wall and the divider
/// and right half are zero-width at the right edge -- the `flex-grow`
/// collapse, which keeps the picture's shape where `scaleX` would distort it.
pub fn layout_rects(rect: Rect, dual: (Rect, Rect, Rect), t: f32) -> (Rect, Rect, Rect) {
    let t = ease(t);
    let (l, d, r) = dual;
    let edge = Rect::from_min_max(pos2(rect.right(), rect.top()), rect.max);
    let mix = |a: Rect, b: Rect| Rect::from_min_max(
        pos2(lerp(a.min.x..=b.min.x, t), a.min.y),
        pos2(lerp(a.max.x..=b.max.x, t), a.max.y),
    );
    (mix(rect, l), mix(edge, d), mix(edge, r))
}

/// One side's dip to black.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Dip {
    /// The input being faded out. None: no dip in progress.
    from: Option<String>,
    /// Black over the picture, 0..1.
    alpha: f32,
    /// Seconds spent holding black, waiting for the new input.
    held: f32,
}

impl Dip {
    /// The side switched away from `from`. A switch during a dip keeps
    /// fading from where it is and waits for the newest input.
    pub fn switch(&mut self, from: Option<String>) {
        if self.from.is_none() {
            self.from = from.or_else(|| Some(String::new()));
            self.alpha = 0.0;
        }
        self.held = 0.0;
    }

    /// Advance; `ready` is true once the new input has something to show.
    pub fn step(&mut self, dt: f32, ready: bool, reduced: bool) {
        if self.from.is_none() {
            return;
        }
        if reduced {
            *self = Self::default();
            return;
        }
        self.alpha = approach(self.alpha, 1.0, dt, DIP_SECS, false);
        if self.alpha >= 1.0 {
            self.held += dt;
            if ready || self.held >= HOLD_MAX_SECS {
                *self = Self::default();
            }
        }
    }

    pub fn active(&self) -> bool {
        self.from.is_some()
    }

    /// The outgoing input, while it is still fading (not yet fully black).
    pub fn fading(&self) -> Option<&str> {
        self.from.as_deref().filter(|_| self.alpha < 1.0).filter(|id| !id.is_empty())
    }

    /// How much black to lay over the half.
    pub fn black(&self) -> f32 {
        if self.active() { self.alpha } else { 0.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::vec2;

    #[test]
    fn approach_reverses_where_it_is() {
        let v = approach(0.0, 1.0, 0.16, LAYOUT_SECS, false);
        assert!((v - 0.5).abs() < 1e-6);
        // Reversed half-way: back to 0 takes the remaining half, not a full run.
        let back = approach(v, 0.0, 0.16, LAYOUT_SECS, false);
        assert_eq!(back, 0.0);
        assert_eq!(approach(0.0, 1.0, 0.001, LAYOUT_SECS, true), 1.0);
    }

    #[test]
    fn layout_ends_match_single_and_dual() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(6000.0, 1200.0));
        let dual = crate::app::dual_rects(rect, 60.0);
        assert_eq!(layout_rects(rect, dual, 1.0), dual);
        let (l, d, r) = layout_rects(rect, dual, 0.0);
        assert_eq!(l, rect);
        assert_eq!(d.width(), 0.0);
        assert_eq!(r.width(), 0.0);
        // Half-way the pieces still tile the wall without overlapping.
        let (l, d, r) = layout_rects(rect, dual, 0.5);
        assert!((l.right() - d.left()).abs() < 1e-3 && (d.right() - r.left()).abs() < 1e-3);
        assert_eq!(r.right(), rect.right());
    }

    #[test]
    fn a_dip_fades_out_holds_and_snaps_in() {
        let mut dip = Dip::default();
        dip.switch(Some("a".into()));
        dip.step(0.065, true, false);
        assert_eq!(dip.fading(), Some("a"));
        assert!((dip.black() - 0.5).abs() < 1e-3);
        // Not ready: holds black, showing neither input.
        dip.step(0.1, false, false);
        assert_eq!((dip.fading(), dip.black()), (None, 1.0));
        dip.step(0.5, true, false);
        assert!(!dip.active());
    }

    #[test]
    fn a_second_switch_keeps_fading_and_waits_for_the_newest() {
        let mut dip = Dip::default();
        dip.switch(Some("a".into()));
        dip.step(0.065, false, false);
        dip.switch(Some("b".into()));
        // Still leaving the picture that was on screen, from the same level.
        assert_eq!(dip.fading(), Some("a"));
        assert!((dip.black() - 0.5).abs() < 1e-3);
    }

    #[test]
    fn a_dead_input_snaps_in_after_the_hold() {
        let mut dip = Dip::default();
        dip.switch(None);
        dip.step(DIP_SECS, false, false);
        dip.step(HOLD_MAX_SECS, false, false);
        assert!(!dip.active());
    }

    #[test]
    fn reduced_motion_is_instant() {
        let mut dip = Dip::default();
        dip.switch(Some("a".into()));
        dip.step(0.001, false, true);
        assert!(!dip.active());
    }
}
