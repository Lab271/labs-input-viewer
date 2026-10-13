// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Audio: each input's sound, mixed by the volume of the halves showing it,
//! into the default output (ADR 0009, ADR 0010).
//!
//! **One capture per device, never per side.** Electron opened audio with
//! each half's video stream, so one input on both halves played twice (E12).
//! Here a capture is keyed by the video device it belongs to, exactly like
//! the video streams (ADR 0008), so that cannot happen: there is nothing to
//! open twice.
//!
//! **Gain rule.** Volume belongs to a half (ADR 0010). A device's gain is
//! the **largest** volume among the *audible* halves showing it, times the
//! output volume:
//!
//! - dual view: both halves are audible; one input on both plays once, at
//!   the louder half's volume -- the half you turned up is the one you hear;
//! - single view: only the left half is on screen, so only it is audible;
//! - a device on no audible half is not captured at all.
//!
//! Max rather than sum because a sum is E12 again (double the level of one
//! input on two halves), and max rather than average because turning one
//! half's copy down must not make the other half's quieter.
//!
//! **Audio never interferes with video.** Every capture and the output run on
//! their own threads (cpal's callbacks plus one holder thread each), share no
//! lock with anything on the video path, and talk to the mixer through
//! bounded lock-free rings that drop rather than grow (`mix.rs`). Device
//! enumeration runs off the UI thread. A device that cannot be found or
//! opened, or vanishes, is logged and retried later; video carries on either
//! way: "missing audio never blocks video".
//!
//! **Mock audio.** With `--mock` each mock input gets a synthetic sine tone,
//! a different pitch per input, presented as a WASAPI-shaped audio device
//! (`Digital Audio Interface (Mock Input N)`) so it goes through the same
//! pairing rule a capture card's audio does.
//!
//! The capsule's Output volume is the app's own master gain (`outputVolume`).
//! Electron's slider set the **OS** master volume (`systemVolume`); that is
//! not ported, and `systemVolume` is kept untouched in settings.json.

mod mix;
pub mod pairing;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use crate::filelog::{log_error, log_info, log_warn};
use mix::{AtomicF32, Frame, Level, MIX_RATE, Mixer, Resampler, Source};
use pairing::{AudioInput, Why};

/// How often the audio inputs are listed again, for hot-plug.
const ENUMERATE_EVERY: Duration = Duration::from_secs(10);
/// A capture that failed or vanished is tried again after this.
const RETRY_AFTER: Duration = Duration::from_secs(5);
/// A gain change is logged once it has held this long, so a slider drag is
/// one line, not sixty.
const GAIN_LOG_SETTLE: Duration = Duration::from_millis(500);
/// Mock tones: an A-major arpeggio, so inputs are told apart by ear.
const MOCK_TONES: [f32; 8] = [220.0, 277.18, 329.63, 440.0, 554.37, 659.25, 880.0, 1108.73];
/// -14 dBFS peak, -17 dBFS RMS: clearly audible, never near clipping.
const MOCK_AMPLITUDE: f32 = 0.2;
pub const MOCK_AUDIO_PREFIX: &str = "mock-audio-";

/// The gain of every device that should be heard: the largest volume among
/// the audible halves showing it, times `output`. In order of first
/// appearance. `audible` is the halves on screen: `[0, 1]` dual, `[0]` single.
pub fn device_gains(sides: &[Option<String>; 2], audible: &[usize], volumes: [f32; 2], output: f32) -> Vec<(String, f32)> {
    let mut gains: Vec<(String, f32)> = Vec::new();
    for &i in audible {
        let Some(id) = sides.get(i).and_then(Option::as_ref) else { continue };
        let v = volumes[i].clamp(0.0, 1.0);
        match gains.iter_mut().find(|(d, _)| d == id) {
            Some((_, g)) => *g = g.max(v),
            None => gains.push((id.clone(), v)),
        }
    }
    let output = output.clamp(0.0, 1.0);
    for (_, g) in &mut gains {
        *g *= output;
    }
    gains
}

/// What a capture reads from.
#[derive(Debug, Clone, PartialEq)]
enum Spec {
    Tone(f32),
    Device { id: String, name: String },
}

/// One open capture. Dropping it stops the thread, which drops the ring's
/// producer, which tells the mixer to let go of the consumer.
struct Capture {
    spec: Spec,
    gain: Arc<AtomicF32>,
    stop: Arc<AtomicBool>,
    /// Set by the capture thread when it ends on its own (failed or vanished).
    failed: Arc<Mutex<Option<String>>>,
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// A gain change waiting to be logged.
struct GainLog {
    logged: f32,
    pending: f32,
    since: Instant,
}

pub struct Engine {
    mock: bool,
    to_mixer: mpsc::Sender<Source>,
    level: Arc<Level>,
    stop_output: Arc<AtomicBool>,
    inputs: Vec<AudioInput>,
    enumerating: Option<mpsc::Receiver<Vec<AudioInput>>>,
    next_enumerate: Instant,
    /// What the pairing was computed from, to redo it only on a change.
    pairing_key: Vec<String>,
    pairs: HashMap<String, (usize, Why)>,
    captures: HashMap<String, Capture>,
    retry_at: HashMap<String, Instant>,
    gain_log: HashMap<String, GainLog>,
    level_every: Duration,
    next_level_log: Instant,
    last_frames: u64,
}

impl Engine {
    /// Start the output. `level_every`: how often the mixed level is logged.
    pub fn start(mock: bool, level_every: Duration) -> Self {
        let (to_mixer, from_engine) = mpsc::channel();
        let level = Arc::new(Level::default());
        let stop_output = Arc::new(AtomicBool::new(false));
        let mixer = Mixer::new(from_engine, level.clone());
        let stop = stop_output.clone();
        if let Err(e) = std::thread::Builder::new().name("audio output".into()).spawn(move || run_output(mixer, &stop)) {
            log_error!("[Audio] could not start the output thread: {e}");
        }
        Self {
            mock,
            to_mixer,
            level,
            stop_output,
            inputs: Vec::new(),
            enumerating: None,
            next_enumerate: Instant::now(),
            pairing_key: Vec::new(),
            pairs: HashMap::new(),
            captures: HashMap::new(),
            retry_at: HashMap::new(),
            gain_log: HashMap::new(),
            level_every,
            next_level_log: Instant::now() + level_every,
            last_frames: 0,
        }
    }

    /// Call every frame from `App::logic`. `videos` is every video device,
    /// `(id, label)`; `overrides` the `audioDevice` settings; `wanted` the
    /// output of [`device_gains`]. `label` names a device in the log.
    pub fn sync(
        &mut self,
        videos: &[(&str, &str)],
        overrides: &HashMap<String, String>,
        wanted: &[(String, f32)],
    ) {
        let now = Instant::now();
        self.enumerate(videos, now);
        self.repair(videos, overrides);
        let label = |id: &str| videos.iter().find(|(v, _)| *v == id).map_or(id, |(_, l)| *l).to_owned();

        // Ended on their own: log, forget, retry later.
        let failed: Vec<String> = self.captures.iter()
            .filter_map(|(id, c)| c.failed.lock().ok()?.clone().map(|_| id.clone()))
            .collect();
        for id in failed {
            if let Some(c) = self.captures.remove(&id) {
                let why = c.failed.lock().ok().and_then(|f| f.clone()).unwrap_or_default();
                log_warn!("[Audio] {} ({}): {why}; retrying in {}s", short(&id), label(&id), RETRY_AFTER.as_secs());
                self.retry_at.insert(id, now + RETRY_AFTER);
            }
        }

        // Close what is no longer heard, or whose pairing changed.
        let specs: HashMap<&str, Option<Spec>> = wanted.iter().map(|(id, _)| (id.as_str(), self.spec_for(id))).collect();
        self.captures.retain(|id, c| {
            let keep = specs.get(id.as_str()).is_some_and(|s| s.as_ref() == Some(&c.spec));
            if !keep {
                log_info!("[Audio] closed {} ({})", short(id), label(id));
            }
            keep
        });
        self.gain_log.retain(|id, _| specs.contains_key(id.as_str()));

        for (id, gain) in wanted {
            let Some(Some(spec)) = specs.get(id.as_str()) else { continue };
            match self.captures.get(id) {
                Some(c) => c.gain.set(*gain),
                None => {
                    if self.retry_at.get(id).is_some_and(|t| now < *t) {
                        continue;
                    }
                    self.retry_at.remove(id);
                    let capture = self.open(id, spec.clone(), *gain);
                    log_info!("[Audio] opened {} ({}) from {} at {}%", short(id), label(id), describe(spec), pct(*gain));
                    self.gain_log.insert(id.clone(), GainLog { logged: *gain, pending: *gain, since: now });
                    self.captures.insert(id.clone(), capture);
                }
            }
            if let Some(g) = self.gain_log.get_mut(id) {
                if g.pending != *gain {
                    g.pending = *gain;
                    g.since = now;
                } else if g.logged != *gain && now.duration_since(g.since) >= GAIN_LOG_SETTLE {
                    log_info!("[Audio] gain {} ({}): {}% -> {}%", short(id), label(id), pct(g.logged), pct(*gain));
                    g.logged = *gain;
                }
            }
        }
        self.log_level(now, &label);
    }

    /// The mixed level, every `level_every`: the one way to tell from a log
    /// that sound is really leaving the app, and at what gain.
    fn log_level(&mut self, now: Instant, label: &dyn Fn(&str) -> String) {
        if now < self.next_level_log {
            return;
        }
        self.next_level_log = now + self.level_every;
        let frames = self.level.frames.load(Ordering::Relaxed);
        let running = frames != self.last_frames;
        self.last_frames = frames;
        let mut playing: Vec<String> = self.captures.iter()
            .map(|(id, c)| format!("{} ({}) {}%", short(id), label(id), pct(c.gain.get())))
            .collect();
        playing.sort();
        if !running {
            if !playing.is_empty() {
                log_warn!("[Audio] output not running; would play {}", playing.join(", "));
            }
            return;
        }
        log_info!("[Audio] output rms {} dBFS, peak {} dBFS, {} mixed; {}",
            mix::dbfs(self.level.rms.get()), mix::dbfs(self.level.peak.get()),
            self.level.sources.load(Ordering::Relaxed),
            if playing.is_empty() { "nothing playing".to_owned() } else { playing.join(", ") });
    }

    /// Real devices: list the audio inputs off the UI thread, now and then.
    /// Mock: one tone per mock input, shaped like a WASAPI endpoint name.
    fn enumerate(&mut self, videos: &[(&str, &str)], now: Instant) {
        if self.mock {
            let list: Vec<AudioInput> = videos.iter()
                .filter_map(|(id, label)| {
                    let n = id.strip_prefix(crate::capture::MOCK_PREFIX)?;
                    Some(AudioInput { id: format!("{MOCK_AUDIO_PREFIX}{n}"), name: format!("Digital Audio Interface ({label})") })
                })
                .collect();
            self.set_inputs(list);
            return;
        }
        if let Some(rx) = &self.enumerating
            && let Ok(list) = rx.try_recv()
        {
            self.enumerating = None;
            self.set_inputs(list);
        }
        if self.enumerating.is_none() && now >= self.next_enumerate {
            self.next_enumerate = now + ENUMERATE_EVERY;
            let (tx, rx) = mpsc::channel();
            self.enumerating = Some(rx);
            let _ = std::thread::Builder::new().name("audio enumerate".into()).spawn(move || {
                let _ = tx.send(device::inputs());
            });
        }
    }

    fn set_inputs(&mut self, list: Vec<AudioInput>) {
        if list != self.inputs {
            log_info!("[Audio] inputs: {}", if list.is_empty() { "none".to_owned() } else {
                list.iter().map(|a| format!("\"{}\"", a.name)).collect::<Vec<_>>().join(", ")
            });
            self.inputs = list;
        }
    }

    /// Recompute the pairing when the devices or overrides changed, and log it.
    fn repair(&mut self, videos: &[(&str, &str)], overrides: &HashMap<String, String>) {
        let mut key: Vec<String> = videos.iter().map(|(id, l)| format!("{id}\u{1}{l}")).collect();
        key.extend(self.inputs.iter().map(|a| format!("{}\u{1}{}", a.id, a.name)));
        let mut o: Vec<_> = overrides.iter().map(|(k, v)| format!("{k}\u{1}{v}")).collect();
        o.sort();
        key.extend(o);
        if key == self.pairing_key {
            return;
        }
        self.pairing_key = key;
        self.pairs = pairing::pair(videos, &self.inputs, overrides);
        for (id, label) in videos {
            match self.pairs.get(*id) {
                Some((a, why)) => log_info!("[Audio] \"{label}\" ({}) pairs with \"{}\" ({})", short(id), self.inputs[*a].name, match why {
                    Why::Override => "audioDevice setting",
                    Why::Name => "same name",
                    Why::NameAnyInstance => "same name, instance unmatched -- check it",
                    Why::Words => "name contained",
                }),
                None => log_info!("[Audio] \"{label}\" ({}) has no audio device", short(id)),
            }
        }
    }

    fn spec_for(&self, video_id: &str) -> Option<Spec> {
        let (a, _) = self.pairs.get(video_id)?;
        let input = self.inputs.get(*a)?;
        Some(match input.id.strip_prefix(MOCK_AUDIO_PREFIX) {
            Some(n) => Spec::Tone(MOCK_TONES[(n.parse::<usize>().unwrap_or(1).max(1) - 1) % MOCK_TONES.len()]),
            None => Spec::Device { id: input.id.clone(), name: input.name.clone() },
        })
    }

    fn open(&self, id: &str, spec: Spec, gain: f32) -> Capture {
        let (producer, consumer) = rtrb::RingBuffer::new(mix::RING_FRAMES);
        let gain = Arc::new(AtomicF32::new(gain));
        let _ = self.to_mixer.send(Source::new(consumer, gain.clone()));
        let stop = Arc::new(AtomicBool::new(false));
        let failed = Arc::new(Mutex::new(None));
        let (s, st, f) = (spec.clone(), stop.clone(), failed.clone());
        let spawned = std::thread::Builder::new().name(format!("audio {}", short(id))).spawn(move || {
            let result = match s {
                Spec::Tone(freq) => {
                    run_tone(freq, producer, &st);
                    Ok(())
                }
                Spec::Device { id, .. } => device::capture(&id, producer, &st),
            };
            if let Err(e) = result
                && let Ok(mut f) = f.lock()
            {
                *f = Some(e);
            }
        });
        if let Err(e) = spawned
            && let Ok(mut f) = failed.lock()
        {
            *f = Some(format!("could not start its thread: {e}"));
        }
        Capture { spec, gain, stop, failed }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.stop_output.store(true, Ordering::Relaxed);
    }
}

fn pct(g: f32) -> u32 {
    (g * 100.0).round() as u32
}

fn short(id: &str) -> String {
    crate::app::short_id(id)
}

fn describe(spec: &Spec) -> String {
    match spec {
        Spec::Tone(f) => format!("a {f} Hz mock tone"),
        Spec::Device { name, .. } => format!("\"{name}\""),
    }
}

/// `--mock`: a sine tone, paced by the clock so it runs at exactly 48 kHz on
/// average however the thread is scheduled. Pushes what it can and drops the
/// rest, as a capture callback does.
fn run_tone(freq: f32, mut ring: rtrb::Producer<Frame>, stop: &AtomicBool) {
    let start = Instant::now();
    let step = f64::from(freq) / f64::from(MIX_RATE);
    let (mut made, mut phase) = (0u64, 0.0f64);
    while !stop.load(Ordering::Relaxed) {
        let due = (start.elapsed().as_secs_f64() * f64::from(MIX_RATE)) as u64;
        while made < due {
            let s = (phase * std::f64::consts::TAU).sin() as f32 * MOCK_AMPLITUDE;
            phase = (phase + step).fract();
            let _ = ring.push([s, s]);
            made += 1;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Holds the output stream for the app's life. Reopens after the device
/// fails; a change of *default* device is followed by cpal itself.
fn run_output(mixer: Mixer, stop: &AtomicBool) {
    let mixer = Arc::new(Mutex::new(mixer));
    let mut last_error = String::new();
    while !stop.load(Ordering::Relaxed) {
        let failed = Arc::new(AtomicBool::new(false));
        match device::open_output(mixer.clone(), failed.clone()) {
            Ok(held) => {
                last_error.clear();
                while !stop.load(Ordering::Relaxed) && !failed.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(200));
                }
                drop(held);
                log_info!("[Audio] output closed");
                if failed.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_secs(2));
                }
            }
            Err(e) => {
                // Once per distinct error: a PC with no speakers would
                // otherwise log this every few seconds forever.
                if e != last_error {
                    log_warn!("[Audio] no output: {e}; trying again every 5s");
                    last_error = e;
                }
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    }
}

/// Everything that touches cpal.
mod device {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{FromSample, Sample, SampleFormat, SizedSample, StreamConfig};

    /// Every audio input on the default host (WASAPI, CoreAudio).
    pub fn inputs() -> Vec<AudioInput> {
        let host = cpal::default_host();
        match host.input_devices() {
            Ok(devices) => devices
                .filter_map(|d| {
                    let id = d.id().ok()?.to_string();
                    let name = d.description().map_or_else(|_| d.to_string(), |desc| desc.name().to_owned());
                    Some(AudioInput { id, name })
                })
                .collect(),
            Err(e) => {
                log_warn!("[Audio] cannot list inputs: {e}");
                Vec::new()
            }
        }
    }

    /// A stream error that means the stream is gone, as opposed to a glitch.
    fn fatal(e: &cpal::Error) -> bool {
        !matches!(e.kind(), cpal::ErrorKind::Xrun | cpal::ErrorKind::DeviceChanged | cpal::ErrorKind::RealtimeDenied)
    }

    /// Capture `id` into `ring` until `stop`. Returns why it ended early.
    pub fn capture(id: &str, ring: rtrb::Producer<Frame>, stop: &AtomicBool) -> Result<(), String> {
        let host = cpal::default_host();
        let parsed: cpal::DeviceId = id.parse().map_err(|e| format!("bad device id {id}: {e}"))?;
        let device = host.device_by_id(&parsed).ok_or("device not found")?;
        let config = device.default_input_config().map_err(|e| e.to_string())?;
        let ended = Arc::new(Mutex::new(None::<String>));
        let dropped = Arc::new(AtomicU64::new(0));
        let sc = config.config();
        let stream = match config.sample_format() {
            SampleFormat::F32 => build_input::<f32>(&device, sc, ring, ended.clone(), dropped.clone()),
            SampleFormat::I16 => build_input::<i16>(&device, sc, ring, ended.clone(), dropped.clone()),
            SampleFormat::I32 => build_input::<i32>(&device, sc, ring, ended.clone(), dropped.clone()),
            SampleFormat::U16 => build_input::<u16>(&device, sc, ring, ended.clone(), dropped.clone()),
            SampleFormat::I8 => build_input::<i8>(&device, sc, ring, ended.clone(), dropped.clone()),
            SampleFormat::U8 => build_input::<u8>(&device, sc, ring, ended.clone(), dropped.clone()),
            SampleFormat::F64 => build_input::<f64>(&device, sc, ring, ended.clone(), dropped.clone()),
            other => return Err(format!("unsupported sample format {other}")),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        log_info!("[Audio] capture \"{}\": {} Hz, {} ch, {}", device, sc.sample_rate, sc.channels, config.sample_format());
        while !stop.load(Ordering::Relaxed) {
            if let Some(e) = ended.lock().ok().and_then(|e| e.clone()) {
                return Err(e);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }

    fn build_input<T>(
        device: &cpal::Device,
        config: StreamConfig,
        mut ring: rtrb::Producer<Frame>,
        ended: Arc<Mutex<Option<String>>>,
        dropped: Arc<AtomicU64>,
    ) -> Result<cpal::Stream, cpal::Error>
    where
        T: SizedSample,
        f32: FromSample<T>,
    {
        let channels = usize::from(config.channels).max(1);
        let mut resampler = Resampler::new(config.sample_rate, MIX_RATE);
        device.build_input_stream::<T, _, _>(
            config,
            move |data: &[T], _| {
                for frame in data.chunks(channels) {
                    let two = [frame[0], *frame.get(1).unwrap_or(&frame[0])].map(|x| x.to_sample::<f32>());
                    let stereo = mix::to_stereo(&two[..frame.len().min(2)]);
                    resampler.push(stereo, |f| {
                        // Full: the mixer is not keeping up (or the output is
                        // gone). Drop, never grow.
                        if ring.push(f).is_err() {
                            dropped.fetch_add(1, Ordering::Relaxed);
                        }
                    });
                }
            },
            move |e| {
                if fatal(&e) && let Ok(mut slot) = ended.lock() {
                    *slot = Some(e.to_string());
                }
            },
            None,
        )
    }

    /// The default output device, playing the mixer. The returned stream must
    /// be held; dropping it stops the sound.
    pub fn open_output(mixer: Arc<Mutex<Mixer>>, failed: Arc<AtomicBool>) -> Result<cpal::Stream, String> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("no default output device")?;
        let config = device.default_output_config().map_err(|e| e.to_string())?;
        let sc = config.config();
        let stream = match config.sample_format() {
            SampleFormat::F32 => build_output::<f32>(&device, sc, mixer, failed),
            SampleFormat::I16 => build_output::<i16>(&device, sc, mixer, failed),
            SampleFormat::I32 => build_output::<i32>(&device, sc, mixer, failed),
            SampleFormat::U16 => build_output::<u16>(&device, sc, mixer, failed),
            SampleFormat::F64 => build_output::<f64>(&device, sc, mixer, failed),
            other => return Err(format!("unsupported output sample format {other}")),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        log_info!("[Audio] output open: \"{}\", {} Hz, {} ch, {}", device, sc.sample_rate, sc.channels, config.sample_format());
        Ok(stream)
    }

    fn build_output<T>(
        device: &cpal::Device,
        config: StreamConfig,
        mixer: Arc<Mutex<Mixer>>,
        failed: Arc<AtomicBool>,
    ) -> Result<cpal::Stream, cpal::Error>
    where
        T: SizedSample + FromSample<f32>,
    {
        let channels = usize::from(config.channels).max(1);
        let rate = config.sample_rate;
        // Integer devices: mixed as f32 here, then converted. Grown once, on
        // the first callback, not every time.
        let mut scratch: Vec<f32> = Vec::new();
        device.build_output_stream::<T, _, _>(
            config,
            move |out: &mut [T], _| {
                // Only this callback locks the mixer once running; a failed
                // try_lock (a reopen handing it over) plays one buffer of
                // silence rather than wait on the real-time thread.
                let Ok(mut m) = mixer.try_lock() else {
                    out.fill(T::EQUILIBRIUM);
                    return;
                };
                if scratch.len() < out.len() {
                    scratch.resize(out.len(), 0.0);
                }
                let buf = &mut scratch[..out.len()];
                m.fill(buf, channels, rate);
                for (o, s) in out.iter_mut().zip(buf.iter()) {
                    *o = s.to_sample::<T>();
                }
            },
            move |e| {
                if fatal(&e) {
                    log_warn!("[Audio] output error: {e}");
                    failed.store(true, Ordering::Relaxed);
                } else if e.kind() == cpal::ErrorKind::DeviceChanged {
                    log_info!("[Audio] output followed the default device: {e}");
                }
            },
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sides(l: Option<&str>, r: Option<&str>) -> [Option<String>; 2] {
        [l.map(str::to_owned), r.map(str::to_owned)]
    }

    #[test]
    fn each_half_sets_its_own_inputs_gain() {
        let g = device_gains(&sides(Some("a"), Some("b")), &[0, 1], [0.8, 0.3], 1.0);
        assert_eq!(g, [("a".to_owned(), 0.8), ("b".to_owned(), 0.3)]);
    }

    #[test]
    fn one_input_on_both_halves_plays_once_at_the_louder_volume() {
        // E12: never two entries for one device.
        let g = device_gains(&sides(Some("a"), Some("a")), &[0, 1], [0.2, 0.6], 1.0);
        assert_eq!(g, [("a".to_owned(), 0.6)]);
    }

    #[test]
    fn single_view_hears_only_the_left_half() {
        let g = device_gains(&sides(Some("a"), Some("b")), &[0], [0.5, 1.0], 1.0);
        assert_eq!(g, [("a".to_owned(), 0.5)]);
    }

    #[test]
    fn output_volume_scales_everything_and_zero_is_mute() {
        let g = device_gains(&sides(Some("a"), Some("b")), &[0, 1], [1.0, 0.5], 0.5);
        assert_eq!(g, [("a".to_owned(), 0.5), ("b".to_owned(), 0.25)]);
        let muted = device_gains(&sides(Some("a"), None), &[0, 1], [0.0, 1.0], 1.0);
        assert_eq!(muted, [("a".to_owned(), 0.0)], "muted, but still captured so unmuting is instant");
        let out_of_range = device_gains(&sides(Some("a"), None), &[0], [3.0, 1.0], -1.0);
        assert_eq!(out_of_range, [("a".to_owned(), 0.0)]);
    }

    #[test]
    fn a_tone_runs_at_the_mix_rate() {
        let (p, mut c) = rtrb::RingBuffer::new(mix::RING_FRAMES);
        let stop = Arc::new(AtomicBool::new(false));
        let st = stop.clone();
        let t = std::thread::spawn(move || run_tone(440.0, p, &st));
        std::thread::sleep(Duration::from_millis(100));
        stop.store(true, Ordering::Relaxed);
        t.join().unwrap();
        let n = c.slots();
        // ~100 ms at 48 kHz, allowing for a slow CI machine.
        assert!((3_000..=mix::RING_FRAMES).contains(&n), "{n}");
        let peak = (0..n).map(|_| c.pop().unwrap()[0].abs()).fold(0.0, f32::max);
        assert!((peak - MOCK_AMPLITUDE).abs() < 0.01, "{peak}");
        assert!(c.is_abandoned());
    }
}
