// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! No-signal detection, simplified.
//!
//! The Electron app compares frames against operator-captured references
//! (detection-simple.js) and separately watches for stalls and flat feeds
//! (stream-health.js). This prototype keeps only the evidence that needs no
//! setup: a 32x18 luma thumbnail that stays one flat colour, or a frame counter
//! that stops. It counts frames and never measures motion -- a held slide still
//! arrives at full rate and must not read as no-signal.

use std::time::{Duration, Instant};

pub const THUMB_W: usize = 32;
pub const THUMB_H: usize = 18;
/// Max-min luma spread under which a thumbnail counts as one flat colour.
pub const FLAT_SPREAD: u8 = 6;
pub const FLAT_FOR: Duration = Duration::from_secs(6);
pub const STALL_AFTER: Duration = Duration::from_secs(3);

/// Nearest-neighbour luma thumbnail of an RGBA frame.
pub fn luma_thumbnail(rgba: &[u8], w: usize, h: usize) -> [u8; THUMB_W * THUMB_H] {
    let mut out = [0u8; THUMB_W * THUMB_H];
    if w == 0 || h == 0 || rgba.len() < w * h * 4 {
        return out;
    }
    for ty in 0..THUMB_H {
        for tx in 0..THUMB_W {
            let x = (tx * 2 + 1) * w / (THUMB_W * 2);
            let y = (ty * 2 + 1) * h / (THUMB_H * 2);
            let i = (y * w + x) * 4;
            let (r, g, b) = (rgba[i] as u32, rgba[i + 1] as u32, rgba[i + 2] as u32);
            out[ty * THUMB_W + tx] = ((r * 54 + g * 183 + b * 19) >> 8) as u8;
        }
    }
    out
}

pub fn is_flat(thumb: &[u8]) -> bool {
    let (min, max) = thumb.iter().fold((u8::MAX, 0), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    max.saturating_sub(min) <= FLAT_SPREAD
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    Waiting,
    Stalled,
    Flat,
    Failed,
}

impl Status {
    pub fn is_no_signal(self) -> bool {
        matches!(self, Status::Stalled | Status::Flat | Status::Failed)
    }
}

/// Per-stream health tracker. Feed it every repaint; it is cheap when the
/// frame sequence has not moved.
#[derive(Debug)]
pub struct Health {
    last_seq: u64,
    last_frame_at: Option<Instant>,
    flat_since: Option<Instant>,
    flowing: bool,
}

impl Health {
    pub fn new() -> Self {
        Self { last_seq: 0, last_frame_at: None, flat_since: None, flowing: false }
    }

    /// `frame` is `(seq, thumbnail-or-None-if-unchanged)`.
    pub fn observe(&mut self, now: Instant, seq: Option<u64>, thumb: Option<&[u8]>, failed: bool) -> Status {
        if failed {
            return Status::Failed;
        }
        if let Some(seq) = seq
            && seq != self.last_seq {
                // Only a second distinct frame counts as flowing, so a static
                // virtual camera that sends one frame is not reopened forever.
                self.flowing = self.last_frame_at.is_some();
                self.last_seq = seq;
                self.last_frame_at = Some(now);
                if let Some(t) = thumb {
                    if is_flat(t) {
                        self.flat_since.get_or_insert(now);
                    } else {
                        self.flat_since = None;
                    }
                }
            }
        let Some(last) = self.last_frame_at else { return Status::Waiting };
        if self.flowing && now.duration_since(last) > STALL_AFTER {
            return Status::Stalled;
        }
        match self.flat_since {
            Some(t) if now.duration_since(t) >= FLAT_FOR => Status::Flat,
            _ => Status::Ok,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: usize, h: usize, v: u8) -> Vec<u8> {
        [v, v, v, 255].repeat(w * h)
    }

    #[test]
    fn flat_and_busy_frames() {
        assert!(is_flat(&luma_thumbnail(&solid(64, 36, 3), 64, 36)));
        let bars = crate::capture::draw_mock_frame(1, 0.5, 320, 180);
        assert!(!is_flat(&luma_thumbnail(&bars, 320, 180)));
    }

    #[test]
    fn flat_only_counts_after_six_seconds() {
        let t0 = Instant::now();
        let flat = luma_thumbnail(&solid(64, 36, 0), 64, 36);
        let mut h = Health::new();
        assert_eq!(h.observe(t0, Some(1), Some(&flat), false), Status::Ok);
        assert_eq!(h.observe(t0 + Duration::from_secs(5), Some(2), Some(&flat), false), Status::Ok);
        assert_eq!(h.observe(t0 + Duration::from_secs(6), Some(3), Some(&flat), false), Status::Flat);
    }

    #[test]
    fn a_single_frame_never_stalls_but_a_stopped_feed_does() {
        let t0 = Instant::now();
        let busy = luma_thumbnail(&crate::capture::draw_mock_frame(1, 0.2, 320, 180), 320, 180);
        let mut one = Health::new();
        one.observe(t0, Some(1), Some(&busy), false);
        assert_eq!(one.observe(t0 + Duration::from_secs(60), Some(1), None, false), Status::Ok);

        let mut flowing = Health::new();
        flowing.observe(t0, Some(1), Some(&busy), false);
        flowing.observe(t0 + Duration::from_millis(60), Some(2), Some(&busy), false);
        assert_eq!(flowing.observe(t0 + Duration::from_secs(4), Some(2), None, false), Status::Stalled);
    }

    #[test]
    fn nothing_yet_is_waiting_not_no_signal() {
        let mut h = Health::new();
        let s = h.observe(Instant::now(), None, None, false);
        assert_eq!(s, Status::Waiting);
        assert!(!s.is_no_signal());
    }
}
