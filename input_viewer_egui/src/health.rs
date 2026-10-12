// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Capture-card health: is this input actually working, and should it be
//! reopened? A port of `stream-health.js` (ADRs 0012 and 0013), constants and
//! tests included.
//!
//! The wall's Elgato cards sometimes come up after boot showing black with a
//! live source plugged in, until someone restarts the app. The driver exposes
//! no signal-lock state, so the only evidence is what arrives on the stream.
//! This module turns that evidence into a status and a reopen decision. It is
//! pure -- no clock of its own, `now` is passed in as milliseconds -- so the
//! policy is unit tested; the app owns the sampling and the reopen.
//!
//! | Status | Meaning |
//! |---|---|
//! | `Opening` | stream open, no frame yet |
//! | `Ok` | frames arriving and the picture is not uniform |
//! | `NoFrames` | open for `FIRST_FRAME_TIMEOUT_MS` and not a single frame |
//! | `Stalled` | frames WERE flowing and have stopped for `STALL_MS` |
//! | `Ended` | the stream ended (driver dropped it, device went away) |
//! | `OpenFailed` | the device could not be opened |
//! | `Dark` | frames arriving, but the picture is one flat colour |
//!
//! A frame COUNT, never motion: #159 rejected motion detection because a held
//! slide is pixel-identical for minutes, and a card still delivers it at
//! 60 fps. "Was flowing first" keeps a static virtual camera, which really
//! sends a frame or two, from being called stalled.
//!
//! Reopening a dark feed is safe because its blank moment cannot be seen; the
//! dark cadence is slow because "nothing plugged in all night" is the steady
//! state.
//!
//! These statuses decide **reopening**. Only `OpenFailed` puts the NO SIGNAL
//! board up (see `InputViewer::side_no_signal`): a dark or stalled card on the
//! wall stays black while it is retried, as in Electron.

pub mod limits {
    /// Open but no frame at all after this long: the card is not delivering.
    pub const FIRST_FRAME_TIMEOUT_MS: f64 = 8000.0;
    /// Frames stopped for this long after flowing: stalled.
    pub const STALL_MS: f64 = 6000.0;
    /// A feed must deliver at least this rate once before a stop counts as a stall.
    pub const FLOWING_MIN_FPS: f64 = 5.0;
    /// Uniform picture for this long before it counts as dark.
    pub const DARK_MS: f64 = 6000.0;
    /// Healthy this long and the reopen backoff starts over.
    pub const RECOVERED_RESET_MS: f64 = 60_000.0;
    /// Delays before each reopen of a faulted feed; the last one repeats.
    pub const FAULT_BACKOFF_MS: [f64; 5] = [2000.0, 5000.0, 15_000.0, 30_000.0, 60_000.0];
    /// Delays before each reopen of a dark feed; the last one repeats.
    pub const DARK_BACKOFF_MS: [f64; 6] = [5000.0, 15_000.0, 30_000.0, 60_000.0, 300_000.0, 600_000.0];
    /// Standard deviation of luma (0-255) at or below which a picture is one flat colour.
    pub const UNIFORM_STD_MAX: f64 = 3.0;
    /// How often the app samples each stream (renderer.js samples every 2 s).
    pub const SAMPLE_MS: f64 = 2000.0;
    /// Gap between closing a device and reopening it, "so the driver sees the
    /// device closed rather than a handover" (REOPEN_RELEASE_MS).
    pub const REOPEN_RELEASE_MS: f64 = 500.0;
}

use limits::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LumaStats {
    pub mean: f64,
    pub std: f64,
}

/// Mean and standard deviation of luma over RGBA pixels (Rec. 601 weights;
/// exactness is irrelevant, flatness is the question). Meant for a small
/// downscale, not a full frame.
pub fn luma_stats(rgba: &[u8]) -> LumaStats {
    let n = rgba.len() / 4;
    if n == 0 {
        return LumaStats { mean: 0.0, std: 0.0 };
    }
    let (mut sum, mut sum_sq) = (0.0f64, 0.0f64);
    for p in rgba.as_chunks::<4>().0 {
        let y = 0.299 * p[0] as f64 + 0.587 * p[1] as f64 + 0.114 * p[2] as f64;
        sum += y;
        sum_sq += y * y;
    }
    let mean = sum / n as f64;
    let variance = (sum_sq / n as f64 - mean * mean).max(0.0);
    LumaStats { mean, std: variance.sqrt() }
}

/// One flat colour -- black, or a solid no-signal screen?
pub fn is_uniform(stats: &LumaStats) -> bool {
    stats.std <= UNIFORM_STD_MAX
}

/// Delay before reopen number `attempt` (0-based); the last step repeats.
pub fn backoff_delay(schedule: &[f64], attempt: usize) -> f64 {
    schedule[attempt.min(schedule.len() - 1)]
}

/// Box-averaged RGBA thumbnail of an RGBA frame.
///
/// Averaged, not point-sampled, as Electron's canvas downscale is: a card's
/// "no signal" screen has a noise floor, and point samples would keep that
/// noise and read a dead input as a picture. Each cell averages a 4x4 grid of
/// samples, so a 4K frame costs ~1.3 M reads at 384x216, not 8.3 M.
pub fn thumbnail(rgba: &[u8], w: usize, h: usize, tw: usize, th: usize) -> Vec<u8> {
    let mut out = vec![0u8; tw * th * 4];
    if w == 0 || h == 0 || rgba.len() < w * h * 4 {
        return out;
    }
    const GRID: usize = 4;
    for ty in 0..th {
        let (y0, y1) = (ty * h / th, ((ty + 1) * h / th).max(ty * h / th + 1));
        for tx in 0..tw {
            let (x0, x1) = (tx * w / tw, ((tx + 1) * w / tw).max(tx * w / tw + 1));
            let mut acc = [0u32; 3];
            let mut n = 0u32;
            for gy in 0..GRID {
                let y = y0 + (y1 - y0) * (2 * gy + 1) / (2 * GRID);
                for gx in 0..GRID {
                    let x = x0 + (x1 - x0) * (2 * gx + 1) / (2 * GRID);
                    let i = (y * w + x) * 4;
                    acc[0] += rgba[i] as u32;
                    acc[1] += rgba[i + 1] as u32;
                    acc[2] += rgba[i + 2] as u32;
                    n += 1;
                }
            }
            let o = (ty * tw + tx) * 4;
            out[o..o + 4].copy_from_slice(&[(acc[0] / n) as u8, (acc[1] / n) as u8, (acc[2] / n) as u8, 255]);
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Idle,
    Opening,
    Ok,
    NoFrames,
    Stalled,
    Ended,
    OpenFailed,
    Dark,
}

impl Status {
    pub fn is_fault(self) -> bool {
        matches!(self, Status::NoFrames | Status::Stalled | Status::Ended | Status::OpenFailed)
    }

    /// The name stream-health.js uses, for log lines that read the same.
    pub fn name(self) -> &'static str {
        match self {
            Status::Idle => "idle",
            Status::Opening => "opening",
            Status::Ok => "ok",
            Status::NoFrames => "no-frames",
            Status::Stalled => "stalled",
            Status::Ended => "ended",
            Status::OpenFailed => "open-failed",
            Status::Dark => "dark",
        }
    }
}

/// One observation of a stream.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    /// Monotonically increasing count of frames delivered, or None if unknown.
    pub frames: Option<u64>,
    pub ended: bool,
    /// Luma of a small downscale, or None.
    pub stats: Option<LumaStats>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decision {
    pub reopen: bool,
    pub reason: Option<Status>,
    pub wait_ms: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Info {
    pub status: Status,
    pub fps: f64,
    pub flowing: bool,
    pub attempts: usize,
    pub luma: Option<LumaStats>,
    pub error: Option<String>,
}

/// Health state for one stream. Feed it events and samples; ask it what to do.
#[derive(Debug, Clone)]
pub struct StreamHealth {
    status: Status,
    opened_at: f64,
    last_frames: Option<u64>,
    last_frame_at: f64,
    last_sample_at: f64,
    first_frame_at: Option<f64>,
    flowing: bool,
    uniform_since: Option<f64>,
    last_stats: Option<LumaStats>,
    fps: f64,
    error: Option<String>,
    // Survive reopens, so the backoff actually backs off.
    attempts: usize,
    last_reopen_at: Option<f64>,
    healthy_since: Option<f64>,
}

impl Default for StreamHealth {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamHealth {
    pub fn new() -> Self {
        Self {
            status: Status::Idle,
            opened_at: 0.0,
            last_frames: None,
            last_frame_at: 0.0,
            last_sample_at: 0.0,
            first_frame_at: None,
            flowing: false,
            uniform_since: None,
            last_stats: None,
            fps: 0.0,
            error: None,
            attempts: 0,
            last_reopen_at: None,
            healthy_since: None,
        }
    }

    fn set_status(&mut self, next: Status, now: f64) {
        self.status = next;
        if next == Status::Ok {
            self.healthy_since.get_or_insert(now);
        } else {
            self.healthy_since = None;
        }
    }

    pub fn status(&self) -> Status {
        self.status
    }

    /// A stream was just attached.
    pub fn opened(&mut self, now: f64) {
        self.opened_at = now;
        self.last_frames = None;
        self.last_frame_at = now;
        self.last_sample_at = now;
        self.first_frame_at = None;
        self.flowing = false;
        self.uniform_since = None;
        self.last_stats = None;
        self.fps = 0.0;
        self.error = None;
        self.set_status(Status::Opening, now);
    }

    /// The device could not be opened.
    pub fn open_failed(&mut self, now: f64, error: &str) {
        self.opened_at = now;
        self.error = Some(error.to_owned());
        self.set_status(Status::OpenFailed, now);
    }

    /// One observation; returns the status after it.
    pub fn sample(&mut self, s: Sample, now: f64) -> Status {
        if matches!(self.status, Status::Idle | Status::OpenFailed) {
            return self.status;
        }
        if s.ended {
            self.set_status(Status::Ended, now);
            return self.status;
        }

        if let Some(frames) = s.frames {
            match self.last_frames {
                None => {
                    self.last_frames = Some(frames);
                    // A counter already past 0 at first look has delivered.
                    if frames > 0 {
                        self.last_frame_at = now;
                        self.first_frame_at = Some(now);
                    }
                }
                Some(last) if frames > last => {
                    let dt = now - self.last_sample_at;
                    if dt > 0.0 {
                        self.fps = (frames - last) as f64 * 1000.0 / dt;
                    }
                    if self.fps >= FLOWING_MIN_FPS {
                        self.flowing = true;
                    }
                    self.first_frame_at.get_or_insert(now);
                    self.last_frames = Some(frames);
                    self.last_frame_at = now;
                }
                // Backwards: the counter restarted with a new source. A new
                // baseline, not evidence of a stall.
                Some(last) if frames < last => {
                    self.last_frames = Some(frames);
                    self.last_frame_at = now;
                }
                Some(_) => self.fps = 0.0,
            }
        }
        self.last_sample_at = now;

        if let Some(stats) = s.stats {
            self.last_stats = Some(stats);
            if is_uniform(&stats) {
                self.uniform_since.get_or_insert(now);
            } else {
                self.uniform_since = None;
            }
        }

        if s.frames.is_some() {
            if self.first_frame_at.is_none() {
                let next = if now - self.opened_at >= FIRST_FRAME_TIMEOUT_MS { Status::NoFrames } else { Status::Opening };
                self.set_status(next, now);
                return self.status;
            }
            if self.flowing && now - self.last_frame_at >= STALL_MS {
                self.set_status(Status::Stalled, now);
                return self.status;
            }
        }

        if self.uniform_since.is_some_and(|t| now - t >= DARK_MS) {
            self.set_status(Status::Dark, now);
            return self.status;
        }

        self.set_status(Status::Ok, now);
        if self.attempts > 0 && self.healthy_since.is_some_and(|t| now - t >= RECOVERED_RESET_MS) {
            self.attempts = 0;
            self.last_reopen_at = None;
        }
        self.status
    }

    /// Should this stream be reopened now?
    pub fn decide(&self, now: f64) -> Decision {
        let schedule: &[f64] = if self.status.is_fault() {
            &FAULT_BACKOFF_MS
        } else if self.status == Status::Dark {
            &DARK_BACKOFF_MS
        } else {
            return Decision { reopen: false, reason: None, wait_ms: 0.0 };
        };
        // From the last reopen, or from the open for the first attempt -- so a
        // stream that goes bad an hour in waits the first delay, not zero.
        let since = self.last_reopen_at.unwrap_or(self.opened_at);
        let wait_ms = (since + backoff_delay(schedule, self.attempts) - now).max(0.0);
        Decision { reopen: wait_ms == 0.0, reason: Some(self.status), wait_ms }
    }

    /// A reopen was started.
    pub fn reopening(&mut self, now: f64) {
        self.attempts += 1;
        self.last_reopen_at = Some(now);
    }

    pub fn info(&self) -> Info {
        Info {
            status: self.status,
            fps: (self.fps * 10.0).round() / 10.0,
            flowing: self.flowing,
            attempts: self.attempts,
            luma: self.last_stats,
            error: self.error.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Ported from test/stream-health.test.js, case for case.
    use super::*;

    const BUSY: LumaStats = LumaStats { mean: 90.0, std: 40.0 };
    const FLAT: LumaStats = LumaStats { mean: 1.0, std: 0.2 };

    /// Drive a tracker at `fps` for `ms`, sampling every `step` ms.
    fn run(h: &mut StreamHealth, from: f64, ms: f64, fps: f64, stats: LumaStats, frames: &mut f64) -> (f64, Status) {
        let step = 2000.0;
        let (mut t, mut status) = (from, h.status());
        while t < from + ms {
            t += step;
            *frames += fps * step / 1000.0;
            status = h.sample(Sample { frames: Some(*frames as u64), ended: false, stats: Some(stats) }, t);
        }
        (t, status)
    }

    fn sample(frames: Option<u64>, ended: bool, stats: Option<LumaStats>) -> Sample {
        Sample { frames, ended, stats }
    }

    #[test]
    fn flat_picture_is_uniform_and_a_varied_one_is_not() {
        let flat = vec![10u8; 32 * 18 * 4];
        assert!(is_uniform(&luma_stats(&flat)));
        let bars: Vec<u8> = (0..32 * 18).flat_map(|i| { let v = if i % 2 == 1 { 250 } else { 5 }; [v, v, v, 255] }).collect();
        assert!(!is_uniform(&luma_stats(&bars)));
    }

    #[test]
    fn a_solid_colour_that_is_not_black_is_uniform_too() {
        // An Elgato-style solid blue no-signal screen is as dead as black.
        let blue: Vec<u8> = (0..64).flat_map(|_| [0, 0, 200, 255]).collect();
        let s = luma_stats(&blue);
        assert!(s.mean > 20.0);
        assert!(is_uniform(&s));
    }

    #[test]
    fn no_division_by_zero_on_an_empty_buffer() {
        assert_eq!(luma_stats(&[]), LumaStats { mean: 0.0, std: 0.0 });
    }

    #[test]
    fn backoff_repeats_the_last_step() {
        let s = [1.0, 2.0, 3.0];
        let got: Vec<f64> = [0, 1, 2, 3, 99].into_iter().map(|a| backoff_delay(&s, a)).collect();
        assert_eq!(got, [1.0, 2.0, 3.0, 3.0, 3.0]);
    }

    #[test]
    fn ok_while_a_card_delivers_a_real_picture() {
        let mut h = StreamHealth::new();
        h.opened(0.0);
        let (_, status) = run(&mut h, 0.0, 30_000.0, 60.0, BUSY, &mut 0.0);
        assert_eq!(status, Status::Ok);
        assert!(!h.decide(30_000.0).reopen);
    }

    #[test]
    fn ok_on_a_held_slide() {
        // The #159 trap: a still picture at 60 fps is a working card.
        let mut h = StreamHealth::new();
        h.opened(0.0);
        assert_eq!(run(&mut h, 0.0, 60_000.0, 60.0, BUSY, &mut 0.0).1, Status::Ok);
    }

    #[test]
    fn no_frames_when_a_card_opens_and_never_delivers() {
        let mut h = StreamHealth::new();
        h.opened(0.0);
        assert_eq!(h.sample(sample(Some(0), false, None), 4000.0), Status::Opening);
        assert_eq!(h.sample(sample(Some(0), false, None), FIRST_FRAME_TIMEOUT_MS), Status::NoFrames);
    }

    #[test]
    fn stalled_when_frames_stop_after_flowing() {
        let mut h = StreamHealth::new();
        h.opened(0.0);
        let mut frames = 0.0;
        let (t, _) = run(&mut h, 0.0, 10_000.0, 60.0, BUSY, &mut frames);
        h.sample(sample(Some(frames as u64), false, Some(BUSY)), t + 2000.0);
        assert_eq!(h.sample(sample(Some(frames as u64), false, Some(BUSY)), t + STALL_MS), Status::Stalled);
    }

    #[test]
    fn a_counter_going_backwards_is_a_new_baseline() {
        let mut h = StreamHealth::new();
        h.opened(0.0);
        let mut frames = 0.0;
        let (t, _) = run(&mut h, 0.0, 10_000.0, 60.0, BUSY, &mut frames);
        frames = 0.0;
        assert_eq!(run(&mut h, t, 10_000.0, 60.0, BUSY, &mut frames).1, Status::Ok);
    }

    #[test]
    fn a_static_virtual_camera_is_not_stalled() {
        // One frame, then nothing: never reached FLOWING_MIN_FPS.
        let mut h = StreamHealth::new();
        h.opened(0.0);
        h.sample(sample(Some(1), false, Some(BUSY)), 500.0);
        let mut t = 2500.0;
        while t < 60_000.0 {
            assert_eq!(h.sample(sample(Some(1), false, Some(BUSY)), t), Status::Ok);
            t += 2000.0;
        }
    }

    #[test]
    fn ended_as_soon_as_the_stream_ends() {
        let mut h = StreamHealth::new();
        h.opened(0.0);
        assert_eq!(h.sample(sample(Some(10), true, Some(BUSY)), 1000.0), Status::Ended);
    }

    #[test]
    fn dark_after_a_flat_picture_persists_not_on_the_first_flat_sample() {
        let mut h = StreamHealth::new();
        h.opened(0.0);
        let mut frames = 0.0;
        let (t, first) = run(&mut h, 0.0, 2000.0, 60.0, FLAT, &mut frames);
        assert_eq!(first, Status::Ok);
        assert_eq!(run(&mut h, t, DARK_MS, 60.0, FLAT, &mut frames).1, Status::Dark);
    }

    #[test]
    fn works_without_a_frame_counter() {
        let mut h = StreamHealth::new();
        h.opened(0.0);
        assert_eq!(h.sample(sample(None, false, Some(BUSY)), 2000.0), Status::Ok);
        h.sample(sample(None, false, Some(FLAT)), 4000.0);
        assert_eq!(h.sample(sample(None, false, Some(FLAT)), 4000.0 + DARK_MS), Status::Dark);
    }

    #[test]
    fn backs_off_between_reopens_of_a_faulted_feed() {
        let mut h = StreamHealth::new();
        h.open_failed(0.0, "NotReadableError");
        assert_eq!(h.info().error.as_deref(), Some("NotReadableError"));
        let mut delays = Vec::new();
        let mut t = 0.0;
        for _ in 0..FAULT_BACKOFF_MS.len() + 2 {
            let d = h.decide(t);
            assert!(!d.reopen);
            t += d.wait_ms;
            let now = h.decide(t);
            assert!(now.reopen);
            assert_eq!(now.reason, Some(Status::OpenFailed));
            delays.push(d.wait_ms);
            h.reopening(t);
            h.open_failed(t, "again");
        }
        let last = *FAULT_BACKOFF_MS.last().unwrap();
        let mut want = FAULT_BACKOFF_MS.to_vec();
        want.extend([last, last]);
        assert_eq!(delays, want);
    }

    #[test]
    fn a_dark_feed_uses_the_slower_schedule() {
        let mut h = StreamHealth::new();
        h.opened(0.0);
        let (t, _) = run(&mut h, 0.0, DARK_MS + 2000.0, 60.0, FLAT, &mut 0.0);
        assert_eq!(h.info().status, Status::Dark);
        let d = h.decide(t);
        assert_eq!(d.reason, Some(Status::Dark));
        // First wait counts from when the stream opened.
        assert_eq!(d.wait_ms, (DARK_BACKOFF_MS[0] - t).max(0.0));
    }

    #[test]
    fn backoff_starts_over_once_a_reopened_feed_stays_healthy() {
        let mut h = StreamHealth::new();
        h.open_failed(0.0, "x");
        h.reopening(2000.0);
        h.open_failed(2000.0, "x");
        h.reopening(7000.0);
        h.opened(7000.0);
        assert_eq!(h.info().attempts, 2);
        run(&mut h, 7000.0, RECOVERED_RESET_MS + 4000.0, 60.0, BUSY, &mut 0.0);
        assert_eq!(h.info().attempts, 0);
    }

    #[test]
    fn nothing_for_a_side_with_no_stream() {
        let mut h = StreamHealth::new();
        assert_eq!(h.sample(sample(Some(0), true, Some(FLAT)), 99_999.0), Status::Idle);
        assert!(!h.decide(99_999.0).reopen);
    }

    // Beyond the JS suite: the thumbnail feeding `stats`.

    #[test]
    fn thumbnail_averages_so_a_noisy_dead_input_reads_flat() {
        // A dark no-signal screen with random noise of 4..16 luma per pixel
        // (std ~3.5): point samples would call it a picture; the 16-sample
        // average per cell brings it to ~0.9, flat.
        let (w, h) = (640, 360);
        let mut rng = crate::screensaver::Rng::new(7);
        let noisy: Vec<u8> = (0..w * h).flat_map(|_| { let v = rng.range(4.0, 16.0) as u8; [v, v, v, 255] }).collect();
        assert!(!is_uniform(&luma_stats(&noisy)));
        assert!(is_uniform(&luma_stats(&thumbnail(&noisy, w, h, 96, 54))));
    }

    #[test]
    fn thumbnail_keeps_a_real_picture_varied() {
        let bars = crate::capture::draw_mock_frame(1, 0.5, 640, 360);
        assert!(!is_uniform(&luma_stats(&thumbnail(&bars, 640, 360, 96, 54))));
    }
}
