// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Capture devices and streams.
//!
//! Each open stream is a thread that writes its latest RGBA frame into a shared
//! slot; the UI thread picks it up once per repaint. A stream is opened per
//! *device*, not per side, so both halves showing one card share it -- the same
//! reason the Electron app releases every side on a device before a reopen.
//!
//! Mock inputs (`--mock`) are generated here rather than stubbed in the UI, so
//! detection reads them through exactly the path a capture card takes.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use eframe::egui;
use crate::filelog::{log_error, log_info, log_warn};

pub const MOCK_PREFIX: &str = "mock-input-";
const MOCK_WIDTH: usize = 1280;
const MOCK_HEIGHT: usize = 720;
const MOCK_FPS: u64 = 15;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// Stable across restarts and replugging: settings are keyed by it.
    pub id: String,
    pub label: String,
    /// Where the backend opens it. Not the id: list positions shift whenever
    /// a device is added, so a setting keyed by position would follow
    /// whichever camera happened to be enumerated there.
    pub index: Option<String>,
    /// Mock inputs only: draw the motion test pattern at this size and rate
    /// instead of colour bars (`--test-pattern`).
    pub test_pattern: Option<PatternFormat>,
    /// Mock inputs only: misbehave this way (`--mock-fault`).
    pub mock_fault: Option<MockFault>,
}

/// How a mock input misbehaves, to drive the health policy without hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockFault {
    /// Frames keep arriving, all one flat dark colour: a card with no source.
    Dark,
    /// Ten seconds of normal frames, then none: a hung card.
    Stall,
    /// The device cannot be opened.
    Fail,
}

/// How long a `Stall` mock delivers before it stops.
const MOCK_STALL_AFTER: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternFormat {
    pub width: usize,
    pub height: usize,
    pub fps: u32,
}

/// A captured frame, already in egui's pixel format.
///
/// Converted on the capture thread, and handed to egui as an `Arc` with no
/// copy. At 3840x2160 a per-pixel conversion on the UI thread costs several
/// milliseconds a frame per input -- for two 4K60 cards, a large part of the
/// frame budget spent before anything is drawn.
pub struct Frame {
    pub image: Arc<egui::ColorImage>,
    pub seq: u64,
    pub captured_at: Instant,
}

impl Frame {
    pub fn new(width: usize, height: usize, pixels: Vec<egui::Color32>, seq: u64) -> Self {
        Self { image: Arc::new(egui::ColorImage::new([width, height], pixels)), seq, captured_at: Instant::now() }
    }
    pub fn width(&self) -> usize {
        self.image.size[0]
    }
    pub fn height(&self) -> usize {
        self.image.size[1]
    }
    /// The same pixels as RGBA bytes, for detection.
    pub fn rgba(&self) -> &[u8] {
        bytemuck::cast_slice(&self.image.pixels)
    }
}

/// RGBA bytes to pixels, reusing the allocation when it can.
pub fn pixels_from_rgba(rgba: Vec<u8>) -> Vec<egui::Color32> {
    bytemuck::allocation::try_cast_vec(rgba).unwrap_or_else(|(_, v)| {
        v.as_chunks::<4>().0.iter().map(|p| egui::Color32::from_rgba_premultiplied(p[0], p[1], p[2], p[3])).collect()
    })
}

#[derive(Default)]
struct Shared {
    latest: Mutex<Option<Arc<Frame>>>,
    error: Mutex<Option<String>>,
}

pub struct Stream {
    shared: Arc<Shared>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Stream {
    pub fn open(device: &Device, ctx: egui::Context) -> Self {
        let shared = Arc::new(Shared::default());
        let stop = Arc::new(AtomicBool::new(false));
        let (s, st, d) = (shared.clone(), stop.clone(), device.clone());
        let handle = std::thread::Builder::new()
            .name(format!("capture {}", device.id))
            .spawn(move || {
                let result = if let Some(n) = d.id.strip_prefix(MOCK_PREFIX) {
                    let n = n.parse().unwrap_or(1);
                    match d.test_pattern {
                        Some(f) => {
                            run_test_pattern(n, f, &s, &st, &ctx);
                            Ok(())
                        }
                        None => run_mock(n, d.mock_fault, &s, &st, &ctx),
                    }
                } else {
                    run_camera(&d, &s, &st, &ctx)
                };
                if let Err(e) = result {
                    log_error!("[Video] {}: {e}", d.label);
                    *s.error.lock().unwrap() = Some(e);
                    ctx.request_repaint();
                }
            })
            .expect("spawn capture thread");
        Self { shared, stop, handle: Some(handle) }
    }

    pub fn latest(&self) -> Option<Arc<Frame>> {
        self.shared.latest.lock().unwrap().clone()
    }

    pub fn error(&self) -> Option<String> {
        self.shared.error.lock().unwrap().clone()
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Joined off-thread: a camera blocked in frame() can take a frame
        // interval to notice, and the UI must not stall on an input switch.
        if let Some(h) = self.handle.take() {
            std::thread::spawn(move || {
                let _ = h.join();
            });
        }
    }
}

fn publish(shared: &Shared, frame: Frame, ctx: &egui::Context) {
    *shared.latest.lock().unwrap() = Some(Arc::new(frame));
    ctx.request_repaint();
}

// ---------------------------------------------------------------------------
// Mock inputs
// ---------------------------------------------------------------------------

/// `fault`, if any, applies to the last input only, so the others stay
/// healthy to compare against.
pub fn mock_devices(count: usize, test_pattern: Option<PatternFormat>, fault: Option<MockFault>) -> Vec<Device> {
    (1..=count)
        .map(|i| Device {
            id: format!("{MOCK_PREFIX}{i}"),
            label: match test_pattern {
                Some(f) => format!("Test {i} {}x{}@{}", f.width, f.height, f.fps),
                None => format!("Mock Input {i}"),
            },
            index: None,
            test_pattern,
            mock_fault: fault.filter(|_| i == count),
        })
        .collect()
}

const BARS: [[u8; 3]; 7] = [
    [0xc0, 0xc0, 0xc0], [0xc0, 0xc0, 0x00], [0x00, 0xc0, 0xc0], [0x00, 0xc0, 0x00],
    [0xc0, 0x00, 0xc0], [0xc0, 0x00, 0x00], [0x00, 0x00, 0xc0],
];

/// Colour bars plus a sweeping bar, so consecutive frames differ and detection
/// reads the input as live. Each input's bars are rotated by its number, so a
/// glance tells them apart without rasterising a label.
pub fn draw_mock_frame(n: usize, phase: f32, w: usize, h: usize) -> Vec<u8> {
    let mut px = vec![0u8; w * h * 4];
    let bar_h = h * 2 / 3;
    let sweep_w = (w as f32 * 0.012).max(2.0);
    let sweep_x = phase * (w as f32 + sweep_w) - sweep_w;
    for y in 0..h {
        for x in 0..w {
            let mut c = if y < bar_h {
                BARS[(x * BARS.len() / w + n - 1) % BARS.len()]
            } else {
                [0x10, 0x10, 0x14]
            };
            if (x as f32) >= sweep_x && (x as f32) < sweep_x + sweep_w {
                for ch in &mut c {
                    *ch = (*ch as f32 * 0.45 + 255.0 * 0.55) as u8;
                }
            }
            let i = (y * w + x) * 4;
            px[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    px
}

fn run_mock(n: usize, fault: Option<MockFault>, shared: &Shared, stop: &AtomicBool, ctx: &egui::Context) -> Result<(), String> {
    if fault == Some(MockFault::Fail) {
        return Err("could not open (--mock-fault=fail)".into());
    }
    let period = Duration::from_millis(1000 / MOCK_FPS);
    let start = Instant::now();
    let mut seq = 0;
    while !stop.load(Ordering::Relaxed) {
        let stalled = fault == Some(MockFault::Stall) && start.elapsed() >= MOCK_STALL_AFTER;
        if !stalled {
            let pixels = if fault == Some(MockFault::Dark) {
                // The near-black a card shows with nothing plugged in.
                vec![egui::Color32::from_rgb(10, 10, 12); MOCK_WIDTH * MOCK_HEIGHT]
            } else {
                let phase = (start.elapsed().as_secs_f32() / 4.0).fract();
                pixels_from_rgba(draw_mock_frame(n, phase, MOCK_WIDTH, MOCK_HEIGHT))
            };
            seq += 1;
            publish(shared, Frame::new(MOCK_WIDTH, MOCK_HEIGHT, pixels, seq), ctx);
        }
        std::thread::sleep(period);
    }
    Ok(())
}

/// `--test-pattern`: frames on an absolute schedule, so the rate is exactly
/// `fps` on average rather than drifting by the time each frame takes to draw.
/// A frame that cannot be drawn in time is skipped, not queued, the way a
/// capture card drops rather than buffers -- and logged, because then the
/// *source* is the bottleneck and the renderer's numbers mean nothing.
fn run_test_pattern(n: usize, f: PatternFormat, shared: &Shared, stop: &AtomicBool, ctx: &egui::Context) {
    let pattern = crate::testpattern::Pattern::new(f.width, f.height, f.fps, n);
    let period = Duration::from_secs_f64(1.0 / f.fps.max(1) as f64);
    let start = Instant::now();
    let mut late = 0u64;
    let mut frame = 0u64;
    while !stop.load(Ordering::Relaxed) {
        let pixels = pattern.frame(frame);
        // seq starts at 1 like every other source; the pattern counts from 0.
        publish(shared, Frame::new(f.width, f.height, pixels, frame + 1), ctx);
        let next = frame + 1;
        let due = start + period.mul_f64(next as f64);
        let now = Instant::now();
        if now < due {
            std::thread::sleep(due - now);
            frame = next;
        } else {
            // Behind: jump to the frame that is due now.
            let behind = ((now - start).as_secs_f64() / period.as_secs_f64()) as u64;
            late += behind.saturating_sub(next);
            frame = behind.max(next);
            if late > 0 && late.is_power_of_two() {
                log_warn!("[TestPattern] input {n}: source fell behind, {late} frames skipped so far");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Real capture devices
// ---------------------------------------------------------------------------

#[cfg(feature = "camera")]
mod camera {
    use super::*;
    use nokhwa::pixel_format::RgbAFormat;
    use nokhwa::utils::{ApiBackend, CameraIndex, RequestedFormat, RequestedFormatType};

    const PREFIX: &str = "camera-";

    /// Ask for camera permission. macOS needs this before `query` returns
    /// anything; elsewhere it completes immediately.
    pub fn initialize(on_done: impl Fn(bool) + Send + Sync + 'static) {
        nokhwa::nokhwa_initialize(on_done);
    }

    pub fn devices() -> Vec<Device> {
        match nokhwa::query(ApiBackend::Auto) {
            Ok(list) => list
                .into_iter()
                .map(|info| {
                    // misc() is AVFoundation's uniqueID on macOS and the
                    // device's symbolic link on Windows: both survive a
                    // restart and a replug into the same port.
                    let stable = info.misc();
                    let key = if stable.trim().is_empty() { info.index().as_string() } else { stable };
                    Device {
                        id: format!("{PREFIX}{key}"),
                        label: info.human_name(),
                        index: Some(info.index().as_string()),
                        test_pattern: None,
                        mock_fault: None,
                    }
                })
                .collect(),
            Err(e) => {
                log_error!("[Devices] device query failed: {e}");
                Vec::new()
            }
        }
    }

    pub fn run(d: &Device, shared: &Shared, stop: &AtomicBool, ctx: &egui::Context) -> Result<(), String> {
        let raw = d.index.as_deref().ok_or("not a camera")?;
        let index = match raw.parse::<u32>() {
            Ok(i) => CameraIndex::Index(i),
            Err(_) => CameraIndex::String(raw.to_owned()),
        };
        let format = RequestedFormat::new::<RgbAFormat>(RequestedFormatType::AbsoluteHighestResolution);
        let mut cam = nokhwa::Camera::new(index, format).map_err(|e| e.to_string())?;
        cam.open_stream().map_err(|e| e.to_string())?;
        let res = cam.resolution();
        log_info!("[Video] {} stream: {}x{}", d.label, res.width_x, res.height_y);
        let mut seq = 0;
        while !stop.load(Ordering::Relaxed) {
            let buf = cam.frame().map_err(|e| e.to_string())?;
            let img = buf.decode_image::<RgbAFormat>().map_err(|e| e.to_string())?;
            seq += 1;
            let (w, h) = (img.width() as usize, img.height() as usize);
            publish(shared, Frame::new(w, h, pixels_from_rgba(img.into_raw()), seq), ctx);
        }
        let _ = cam.stop_stream();
        Ok(())
    }
}

#[cfg(feature = "camera")]
pub use camera::{devices as camera_devices, initialize as camera_initialize};

#[cfg(feature = "camera")]
fn run_camera(d: &Device, s: &Shared, st: &AtomicBool, ctx: &egui::Context) -> Result<(), String> {
    camera::run(d, s, st, ctx)
}

#[cfg(not(feature = "camera"))]
pub fn camera_initialize(on_done: impl Fn(bool) + Send + Sync + 'static) {
    on_done(false);
}

#[cfg(not(feature = "camera"))]
pub fn camera_devices() -> Vec<Device> {
    Vec::new()
}

#[cfg(not(feature = "camera"))]
fn run_camera(_: &Device, _: &Shared, _: &AtomicBool, _: &egui::Context) -> Result<(), String> {
    Err("built without the `camera` feature".into())
}
