// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Sample conversion, resampling and the mixer -- everything between a
//! capture callback and the output callback, kept free of cpal so it can be
//! tested.
//!
//! Every source is converted on its own callback to one internal format,
//! **stereo f32 at 48 kHz**, and pushed into its own bounded ring. The output
//! callback mixes the rings at 48 kHz and converts once more, to whatever the
//! output device runs at. Two conversions instead of one, but a source never
//! needs to know which output it plays on, so the output device can change
//! (the wall's display is the speaker, and comes and goes with it) without
//! touching a single capture.
//!
//! Each ring holds at most [`RING_FRAMES`] (200 ms). A source that runs ahead
//! of the output -- two clocks never agree exactly -- has its oldest audio
//! dropped down to [`TARGET_FRAMES`] once it passes [`MAX_FRAMES`]; one that
//! runs dry plays silence until [`PRIME_FRAMES`] have built up again. Latency
//! is bounded both ways and nothing grows.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc;

/// The internal rate every source is converted to.
pub const MIX_RATE: u32 = 48_000;
/// Ring capacity per source: 200 ms.
pub const RING_FRAMES: usize = MIX_RATE as usize / 5;
/// A source starts (and restarts after running dry) with 20 ms buffered.
pub const PRIME_FRAMES: usize = MIX_RATE as usize / 50;
/// Past 100 ms buffered the source is ahead of the output...
pub const MAX_FRAMES: usize = MIX_RATE as usize / 10;
/// ...and is trimmed back to 40 ms.
pub const TARGET_FRAMES: usize = MIX_RATE as usize / 25;
/// Gain moves toward its target with a ~10 ms time constant, so a slider drag
/// does not click.
const GAIN_SMOOTHING: f32 = 1.0 / (0.010 * MIX_RATE as f32);
/// How often the output level is published for the log: every 0.5 s.
const STATS_FRAMES: u64 = MIX_RATE as u64 / 2;

pub type Frame = [f32; 2];

/// One frame of any channel count to stereo: mono is duplicated, and of more
/// than two channels the first two (front left and right in every layout a
/// capture card or HDMI source uses) are kept.
pub fn to_stereo(frame: &[f32]) -> Frame {
    match frame {
        [] => [0.0, 0.0],
        [m] => [*m, *m],
        [l, r, ..] => [*l, *r],
    }
}

/// A stereo frame into an output frame of any channel count: mono gets the
/// average, extra channels silence.
pub fn write_frame(out: &mut [f32], f: Frame) {
    match out {
        [] => {}
        [m] => *m = (f[0] + f[1]) * 0.5,
        [l, r, rest @ ..] => {
            *l = f[0];
            *r = f[1];
            rest.fill(0.0);
        }
    }
}

/// Weighted rather than `a + (b - a) * t`, so `t == 1` gives `b` exactly.
fn lerp(a: Frame, b: Frame, t: f32) -> Frame {
    [a[0] * (1.0 - t) + b[0] * t, a[1] * (1.0 - t) + b[1] * t]
}

/// Linear-interpolation sample-rate converter. Not studio quality -- it
/// aliases a little going down -- but this is a room's monitoring, from
/// sources that are 48 kHz already in the common case, where it is exact.
#[derive(Debug, Clone)]
pub struct Resampler {
    /// Input frames per output frame.
    step: f64,
    /// Where the next output frame lies after `prev`, in (0, 1] once running:
    /// 1 is `cur` itself, so equal rates pass every frame through untouched.
    t: f64,
    prev: Frame,
    cur: Frame,
}

impl Resampler {
    /// For a capture callback, driven by [`push`](Self::push).
    pub fn new(from: u32, to: u32) -> Self {
        Self { step: f64::from(from.max(1)) / f64::from(to.max(1)), t: 1.0, prev: [0.0; 2], cur: [0.0; 2] }
    }

    /// For an output callback, driven by [`pull`](Self::pull): starts one
    /// frame further on, so the first pull fetches the first input.
    pub fn for_pull(from: u32, to: u32) -> Self {
        Self { t: 2.0, ..Self::new(from, to) }
    }

    /// Feed one input frame; emits the output frames now due (zero or more).
    pub fn push(&mut self, input: Frame, mut emit: impl FnMut(Frame)) {
        self.prev = self.cur;
        self.cur = input;
        while self.t <= 1.0 {
            emit(lerp(self.prev, self.cur, self.t as f32));
            self.t += self.step;
        }
        self.t -= 1.0;
    }

    /// Produce one output frame, pulling input frames as needed.
    pub fn pull(&mut self, mut next: impl FnMut() -> Frame) -> Frame {
        while self.t > 1.0 {
            self.prev = self.cur;
            self.cur = next();
            self.t -= 1.0;
        }
        let out = lerp(self.prev, self.cur, self.t as f32);
        self.t += self.step;
        out
    }
}

/// An `f32` shared without a lock: the UI writes a gain, the output callback
/// reads it.
#[derive(Debug, Default)]
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    pub fn new(v: f32) -> Self {
        Self(AtomicU32::new(v.to_bits()))
    }
    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
    pub fn set(&self, v: f32) {
        self.0.store(v.to_bits(), Ordering::Relaxed);
    }
}

/// A source as the mixer sees it.
pub struct Source {
    pub ring: rtrb::Consumer<Frame>,
    /// Target gain, set from the UI thread.
    pub gain: Arc<AtomicF32>,
    current: f32,
    primed: bool,
}

impl Source {
    pub fn new(ring: rtrb::Consumer<Frame>, gain: Arc<AtomicF32>) -> Self {
        // Starts at its target rather than ramping up from 0: a source opens
        // at the volume its half already has.
        let current = gain.get();
        Self { ring, gain, current, primed: false }
    }

    /// The next frame at the mix rate, gain applied; silence when not primed.
    fn next(&mut self) -> Frame {
        let buffered = self.ring.slots();
        if !self.primed {
            if buffered < PRIME_FRAMES {
                return [0.0; 2];
            }
            self.primed = true;
        }
        if buffered > MAX_FRAMES {
            // Ahead of the output: drop the oldest audio, never grow.
            for _ in 0..buffered - TARGET_FRAMES {
                let _ = self.ring.pop();
            }
        }
        let Ok(f) = self.ring.pop() else {
            self.primed = false;
            return [0.0; 2];
        };
        self.current += (self.gain.get() - self.current) * GAIN_SMOOTHING;
        [f[0] * self.current, f[1] * self.current]
    }

    /// The producer is gone (capture closed) and everything it sent is played.
    fn finished(&self) -> bool {
        self.ring.is_abandoned() && self.ring.is_empty()
    }
}

/// The output level, published by the output callback twice a second.
#[derive(Debug, Default)]
pub struct Level {
    pub rms: AtomicF32,
    pub peak: AtomicF32,
    pub sources: AtomicU32,
    /// Frames delivered to the output device since start.
    pub frames: AtomicU64,
}

/// Sums the sources at [`MIX_RATE`] and converts to the device's format.
pub struct Mixer {
    sources: Vec<Source>,
    incoming: mpsc::Receiver<Source>,
    out_rate: u32,
    resampler: Resampler,
    level: Arc<Level>,
    sum_sq: f64,
    peak: f32,
    count: u64,
}

impl Mixer {
    pub fn new(incoming: mpsc::Receiver<Source>, level: Arc<Level>) -> Self {
        Self {
            sources: Vec::new(),
            incoming,
            out_rate: MIX_RATE,
            resampler: Resampler::for_pull(MIX_RATE, MIX_RATE),
            level,
            sum_sq: 0.0,
            peak: 0.0,
            count: 0,
        }
    }

    /// One mixed frame at the mix rate, clipped to full scale.
    fn mix_one(sources: &mut [Source]) -> Frame {
        let mut sum = [0.0f32; 2];
        for s in sources {
            let f = s.next();
            sum[0] += f[0];
            sum[1] += f[1];
        }
        [sum[0].clamp(-1.0, 1.0), sum[1].clamp(-1.0, 1.0)]
    }

    /// Fill an interleaved output buffer of `channels` at `rate`. Called on
    /// the output device's real-time thread: no allocation in the steady
    /// state, no locks, no logging.
    pub fn fill(&mut self, out: &mut [f32], channels: usize, rate: u32) {
        while let Ok(s) = self.incoming.try_recv() {
            self.sources.push(s);
        }
        self.sources.retain(|s| !s.finished());
        if rate != self.out_rate {
            self.out_rate = rate;
            self.resampler = Resampler::for_pull(MIX_RATE, rate);
        }
        let channels = channels.max(1);
        let sources = &mut self.sources;
        for frame in out.chunks_mut(channels) {
            let f = if rate == MIX_RATE {
                Self::mix_one(sources)
            } else {
                self.resampler.pull(|| Self::mix_one(sources))
            };
            write_frame(frame, f);
            self.sum_sq += f64::from(f[0] * f[0] + f[1] * f[1]) * 0.5;
            self.peak = self.peak.max(f[0].abs()).max(f[1].abs());
            self.count += 1;
        }
        self.level.frames.fetch_add((out.len() / channels) as u64, Ordering::Relaxed);
        if self.count >= STATS_FRAMES {
            self.level.rms.set((self.sum_sq / self.count as f64).sqrt() as f32);
            self.level.peak.set(self.peak);
            self.level.sources.store(self.sources.len() as u32, Ordering::Relaxed);
            (self.sum_sq, self.peak, self.count) = (0.0, 0.0, 0);
        }
    }
}

/// A level as dBFS for the log; silence as `-inf`.
pub fn dbfs(level: f32) -> String {
    if level <= 1e-6 { "-inf".to_owned() } else { format!("{:.1}", 20.0 * level.log10()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_counts_convert_both_ways() {
        assert_eq!(to_stereo(&[0.5]), [0.5, 0.5]);
        assert_eq!(to_stereo(&[0.1, 0.2, 0.9, 0.9, 0.9, 0.9, 0.9, 0.9]), [0.1, 0.2], "8ch HDMI keeps front L/R");
        assert_eq!(to_stereo(&[]), [0.0, 0.0]);
        let mut mono = [9.0];
        write_frame(&mut mono, [0.2, 0.4]);
        assert!((mono[0] - 0.3).abs() < 1e-6);
        let mut six = [9.0; 6];
        write_frame(&mut six, [0.2, 0.4]);
        assert_eq!(six, [0.2, 0.4, 0.0, 0.0, 0.0, 0.0]);
    }

    fn push_all(r: &mut Resampler, input: impl IntoIterator<Item = f32>) -> Vec<f32> {
        let mut out = Vec::new();
        for x in input {
            r.push([x, -x], |f| out.push(f[0]));
        }
        out
    }

    #[test]
    fn equal_rates_pass_through_exactly() {
        let input: Vec<f32> = (0..100).map(|i| (i as f32 * 0.1).sin()).collect();
        let out = push_all(&mut Resampler::new(48_000, 48_000), input.iter().copied());
        assert_eq!(out, input);
    }

    #[test]
    fn rate_conversion_keeps_duration() {
        // One second of 44.1 kHz becomes one second of 48 kHz, give or take a frame.
        let out = push_all(&mut Resampler::new(44_100, 48_000), (0..44_100).map(|_| 0.0));
        assert!((out.len() as i64 - 48_000).abs() <= 1, "{}", out.len());
        let out = push_all(&mut Resampler::new(96_000, 48_000), (0..96_000).map(|_| 0.0));
        assert!((out.len() as i64 - 48_000).abs() <= 1, "{}", out.len());
    }

    #[test]
    fn interpolation_follows_a_ramp() {
        // Upsampling 2x a ramp gives the midpoints.
        let out = push_all(&mut Resampler::new(24_000, 48_000), (0..5).map(|i| i as f32));
        assert_eq!(out, [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0]);
        // Pull is the same converter run from the other end.
        let mut r = Resampler::for_pull(24_000, 48_000);
        let mut i = 0.0;
        let pulled: Vec<f32> = (0..6).map(|_| r.pull(|| { let f = [i, i]; i += 1.0; f })[0]).collect();
        assert_eq!(pulled, [0.0, 0.5, 1.0, 1.5, 2.0, 2.5]);
    }

    fn source(gain: f32, frames: &[Frame]) -> (Source, rtrb::Producer<Frame>, Arc<AtomicF32>) {
        let (mut p, c) = rtrb::RingBuffer::new(RING_FRAMES);
        for f in frames {
            p.push(*f).unwrap();
        }
        let g = Arc::new(AtomicF32::new(gain));
        (Source::new(c, g.clone()), p, g)
    }

    fn mixer() -> (Mixer, mpsc::Sender<Source>, Arc<Level>) {
        let (tx, rx) = mpsc::channel();
        let level = Arc::new(Level::default());
        (Mixer::new(rx, level.clone()), tx, level)
    }

    #[test]
    fn sources_sum_with_their_gains_and_clip_at_full_scale() {
        let (mut m, tx, _) = mixer();
        let (a, _pa, _) = source(0.5, &[[0.4, 0.4]; PRIME_FRAMES]);
        let (b, _pb, _) = source(1.0, &[[0.1, -0.1]; PRIME_FRAMES]);
        tx.send(a).unwrap();
        tx.send(b).unwrap();
        let mut out = [0.0f32; 4];
        m.fill(&mut out, 2, MIX_RATE);
        assert!((out[0] - 0.3).abs() < 1e-6 && (out[1] - 0.1).abs() < 1e-6, "{out:?}");

        let (mut m, tx, _) = mixer();
        let (loud, _p, _) = source(1.0, &[[0.9, -0.9]; PRIME_FRAMES]);
        let (loud2, _p2, _) = source(1.0, &[[0.9, -0.9]; PRIME_FRAMES]);
        tx.send(loud).unwrap();
        tx.send(loud2).unwrap();
        m.fill(&mut out, 2, MIX_RATE);
        assert_eq!(&out[..2], &[1.0, -1.0]);
    }

    #[test]
    fn a_source_waits_to_prime_and_goes_silent_when_dry() {
        let (mut m, tx, _) = mixer();
        let (s, mut p, _) = source(1.0, &[[0.5, 0.5]; PRIME_FRAMES - 1]);
        tx.send(s).unwrap();
        let mut out = vec![0.0f32; 2 * 8];
        m.fill(&mut out, 2, MIX_RATE);
        assert!(out.iter().all(|&x| x == 0.0), "not primed yet");
        p.push([0.5, 0.5]).unwrap();
        let mut out = vec![0.0f32; 2 * (PRIME_FRAMES + 4)];
        m.fill(&mut out, 2, MIX_RATE);
        assert_eq!(out[0], 0.5);
        assert_eq!(out[2 * PRIME_FRAMES - 2], 0.5, "the last buffered frame");
        assert_eq!(out[2 * PRIME_FRAMES], 0.0, "dry: silence, not a held sample");
    }

    #[test]
    fn a_source_ahead_of_the_output_is_trimmed_not_grown() {
        let (mut m, tx, _) = mixer();
        let frames: Vec<Frame> = (0..RING_FRAMES).map(|i| [i as f32 / RING_FRAMES as f32, 0.0]).collect();
        let (s, _p, _) = source(1.0, &frames);
        tx.send(s).unwrap();
        let mut out = [0.0f32; 2];
        m.fill(&mut out, 2, MIX_RATE);
        // Played from TARGET_FRAMES before the newest, not from the oldest.
        assert_eq!(out[0], (RING_FRAMES - TARGET_FRAMES) as f32 / RING_FRAMES as f32);
    }

    #[test]
    fn a_closed_source_is_dropped_once_played_out() {
        let (mut m, tx, level) = mixer();
        let (s, p, _) = source(1.0, &[[0.1, 0.1]; PRIME_FRAMES]);
        tx.send(s).unwrap();
        drop(p);
        let mut out = vec![0.0f32; 2 * PRIME_FRAMES];
        m.fill(&mut out, 2, MIX_RATE);
        m.fill(&mut out, 2, MIX_RATE);
        assert!(m.sources.is_empty());
        assert_eq!(level.frames.load(Ordering::Relaxed), 2 * PRIME_FRAMES as u64);
    }

    #[test]
    fn gain_changes_glide_rather_than_jump() {
        let (mut m, tx, _) = mixer();
        let (s, _p, g) = source(1.0, &[[1.0, 1.0]; RING_FRAMES / 2]);
        tx.send(s).unwrap();
        g.set(0.0);
        let mut out = vec![0.0f32; 2 * 960];
        m.fill(&mut out, 2, MIX_RATE);
        assert!(out[0] > 0.99, "first frame barely moves: {}", out[0]);
        assert!(out[2 * 959] < 0.2, "20 ms later it is mostly down: {}", out[2 * 959]);
    }

    #[test]
    fn output_at_another_rate_and_channel_count() {
        let (mut m, tx, level) = mixer();
        let (s, _p, _) = source(1.0, &[[0.25, 0.25]; MAX_FRAMES]);
        tx.send(s).unwrap();
        // 44.1 kHz, 6 channels: half a second's worth publishes a level.
        let mut out = vec![0.0f32; 6 * STATS_FRAMES as usize];
        m.fill(&mut out, 6, 44_100);
        assert!(out.chunks(6).skip(4).take(100).all(|f| f[2..].iter().all(|&x| x == 0.0)));
        // The source only had 100 ms; the rest is silence, so the level is
        // its share of 0.25.
        let rms = level.rms.get();
        assert!(rms > 0.05 && rms < 0.25, "{rms}");
        assert_eq!(dbfs(0.0), "-inf");
        assert_eq!(dbfs(1.0), "0.0");
    }
}
