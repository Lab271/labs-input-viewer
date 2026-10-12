// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Per-stream frame accounting for `--hud`: is every frame the source makes
//! reaching the screen?
//!
//! - **source fps** -- from the sequence numbers, so it counts frames the
//!   source made whether or not they were shown.
//! - **shown fps** -- frames actually uploaded to a texture.
//! - **skipped** -- source frames replaced before the UI got to them. Zero
//!   means the renderer keeps up; anything else means frames never reached
//!   the screen.
//! - **delay** -- from the frame being captured to its upload. Not glass to
//!   glass (the paint and the display's scanout come after), but it is the
//!   part this app controls.

use std::time::{Duration, Instant};

pub const WINDOW: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Report {
    pub source_fps: f32,
    pub shown_fps: f32,
    pub skipped: u64,
    pub delay_ms: f32,
}

#[derive(Debug)]
pub struct RenderStats {
    window_start: Option<Instant>,
    first_seq: u64,
    last_seq: u64,
    shown: u32,
    skipped: u64,
    delay: Duration,
    delay_n: u32,
    pub total_skipped: u64,
    pub last: Option<Report>,
}

impl RenderStats {
    pub fn new() -> Self {
        Self {
            window_start: None,
            first_seq: 0,
            last_seq: 0,
            shown: 0,
            skipped: 0,
            delay: Duration::ZERO,
            delay_n: 0,
            total_skipped: 0,
            last: None,
        }
    }

    /// A frame was uploaded. `prev_seq` is the one shown before it (0: none).
    pub fn shown(&mut self, now: Instant, seq: u64, prev_seq: u64, captured_at: Instant) {
        if self.window_start.is_none() {
            self.window_start = Some(now);
            self.first_seq = seq;
        }
        if prev_seq != 0 && seq > prev_seq + 1 {
            let gap = seq - prev_seq - 1;
            self.skipped += gap;
            self.total_skipped += gap;
        }
        self.last_seq = seq;
        self.shown += 1;
        self.delay += now.saturating_duration_since(captured_at);
        self.delay_n += 1;
    }

    /// Close the window once a second; returns the new report when it does.
    pub fn tick(&mut self, now: Instant) -> Option<Report> {
        let start = self.window_start?;
        let elapsed = now.saturating_duration_since(start);
        if elapsed < WINDOW {
            return None;
        }
        let secs = elapsed.as_secs_f32();
        let report = Report {
            source_fps: (self.last_seq - self.first_seq) as f32 / secs,
            shown_fps: self.shown.saturating_sub(1) as f32 / secs,
            skipped: self.skipped,
            delay_ms: self.delay.as_secs_f32() * 1000.0 / self.delay_n.max(1) as f32,
        };
        // The next window starts at the last frame, so no interval is lost.
        // That frame opens the window as a fence post for the rates; its
        // delay was already counted in this one.
        self.window_start = Some(now);
        self.first_seq = self.last_seq;
        self.shown = 1;
        self.skipped = 0;
        self.delay = Duration::ZERO;
        self.delay_n = 0;
        self.last = Some(report);
        Some(report)
    }
}

impl Report {
    pub fn line(&self) -> String {
        format!(
            "source {:.1} fps · shown {:.1} fps · skipped {} · delay {:.1} ms",
            self.source_fps, self.shown_fps, self.skipped, self.delay_ms
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(source_fps: u64, shown_every: u64, secs: u64) -> Report {
        let t0 = Instant::now();
        let mut s = RenderStats::new();
        let mut prev = 0;
        let mut last = None;
        for seq in 1..=source_fps * secs + 1 {
            let at = t0 + Duration::from_secs_f64((seq - 1) as f64 / source_fps as f64);
            if (seq - 1) % shown_every == 0 {
                s.shown(at, seq, prev, at - Duration::from_millis(2));
                prev = seq;
            }
            if let Some(r) = s.tick(at) {
                last = Some(r);
            }
        }
        last.expect("a full window")
    }

    #[test]
    fn keeping_up_shows_every_frame() {
        let r = run(60, 1, 3);
        assert!((r.source_fps - 60.0).abs() < 0.5, "{r:?}");
        assert!((r.shown_fps - 60.0).abs() < 0.5, "{r:?}");
        assert_eq!(r.skipped, 0);
        assert!((r.delay_ms - 2.0).abs() < 0.01);
    }

    #[test]
    fn a_renderer_at_half_rate_skips_every_other_frame() {
        let r = run(60, 2, 3);
        assert!((r.shown_fps - 30.0).abs() < 0.5, "{r:?}");
        assert!((r.source_fps - 60.0).abs() < 0.5, "{r:?}");
        assert_eq!(r.skipped, 30);
    }

    #[test]
    fn nothing_until_a_full_window() {
        let mut s = RenderStats::new();
        let t0 = Instant::now();
        s.shown(t0, 1, 0, t0);
        assert_eq!(s.tick(t0 + Duration::from_millis(500)), None);
    }
}
