// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! No-signal detection against operator-captured references -- a port of
//! `detection-simple.js` (ADR 0014, D1).
//!
//! A capture card with nothing plugged in still sends frames: its own "no
//! signal" screen. The operator captures what that looks like (one card has
//! several: no cable, unsupported mode, HDCP error), and a frame matching
//! ANY reference means no signal. The arithmetic and thresholds are
//! Electron's, unchanged.
//!
//! **It must never interfere with the video (D1).** So:
//!
//! - Comparisons run on their own thread, never the UI or a capture thread.
//! - A job holds the frame by `Arc` -- the frame the capture thread already
//!   made; nothing is read back from the GPU and nothing is copied whole. The
//!   worker samples a 480x270 nearest-neighbour thumbnail out of it.
//! - At most one job per device is in flight, every 1.6 s; a slow worker
//!   skips cycles rather than queueing them.
//! - A device without references sends no jobs at all. The wall has none,
//!   so there it costs nothing.
//!
//! References stay where Electron keeps them, `noSignalReferences` in
//! settings.json, as 480x270 PNG data URLs, so captures made by either app
//! work in the other. The whole frame-source layer of the Electron version
//! (rVFC pacing, WebCodecs, the canvas fallback, the `copyTo` crop trap)
//! has no counterpart: the frame is already on the CPU.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use base64::Engine;
use serde_json::{Map, Value, json};

use crate::capture::{Frame, Pixels};
use crate::filelog::{log_info, log_warn};

/// frame-source.js `DETECT_WIDTH` / `DETECT_HEIGHT`.
pub const DETECT_W: usize = 480;
pub const DETECT_H: usize = 270;
/// Electron's 1600 ms cycle, which keeps going when frames stop.
pub const EVERY: Duration = Duration::from_millis(1600);

/// detection-simple.js CONFIG.
const PIXEL_DIFF: i16 = 30;
const MATCH_RATIO: f32 = 0.95;
const SAMPLE_RATE: usize = 4;
const PROBE_POINTS: usize = 32;
/// ~15% of the probe: a loose match still reaches the full scan, which
/// decides properly. Tuned so the probe never rejects what the scan matches.
const PROBE_MAX_MISSES: usize = 5;

/// An RGBA image at detection size.
#[derive(Clone, PartialEq, Debug)]
pub struct Image {
    pub w: usize,
    pub h: usize,
    pub rgba: Vec<u8>,
}

fn differs(a: &[u8], b: &[u8]) -> bool {
    (0..3).any(|c| (i16::from(a[c]) - i16::from(b[c])).abs() > PIXEL_DIFF)
}

/// `probeFrames`: 32 golden-ratio points, rejecting after more than five
/// misses. Cheap enough to run before every full scan.
pub fn probe(frame: &Image, reference: &Image) -> bool {
    let pixels = frame.w * frame.h;
    if pixels == 0 || frame.rgba.len() != reference.rgba.len() {
        return false;
    }
    let mut misses = 0;
    for i in 0..PROBE_POINTS {
        let px = ((i as f64 * 0.618_033_988_7).fract() * pixels as f64) as usize;
        let o = px * 4;
        if differs(&frame.rgba[o..o + 3], &reference.rgba[o..o + 3]) {
            misses += 1;
            if misses > PROBE_MAX_MISSES {
                return false;
            }
        }
    }
    true
}

/// `compareFrames`: every 4th pixel must be within 30 per channel for 95%
/// of samples, with an early exit once that is arithmetically impossible
/// (#104: 1.47 ms to 0.065 ms at 1080p, for a live picture).
pub fn compare(frame: &Image, reference: &Image) -> bool {
    if frame.w != reference.w || frame.h != reference.h {
        return false;
    }
    let stride = 4 * SAMPLE_RATE;
    let total = frame.rgba.len().div_ceil(stride);
    let max_misses = total - (total as f32 * MATCH_RATIO).ceil() as usize;
    let mut misses = 0;
    for i in (0..frame.rgba.len()).step_by(stride) {
        if differs(&frame.rgba[i..i + 3], &reference.rgba[i..i + 3]) {
            misses += 1;
            if misses > max_misses {
                return false;
            }
        }
    }
    true
}

/// Matching any one reference is no signal.
pub fn matches_any(frame: &Image, references: &[Image]) -> bool {
    references.iter().any(|r| probe(frame, r) && compare(frame, r))
}

/// Two agreeing results flip the state, as Electron's `matchCount` /
/// `noMatchCount`; a single odd frame changes nothing.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Debounce {
    pub no_signal: bool,
    streak: u8,
}

impl Debounce {
    /// Returns true when the state flipped.
    pub fn feed(&mut self, matched: bool) -> bool {
        if matched == self.no_signal {
            self.streak = 0;
            return false;
        }
        self.streak += 1;
        if self.streak >= 2 {
            self.no_signal = matched;
            self.streak = 0;
            return true;
        }
        false
    }
}

/// Nearest-neighbour downscale of a captured frame to at most 480x270, as
/// frame-source.js does, straight from the card's bytes.
pub fn thumbnail(frame: &Frame) -> Image {
    let (fw, fh) = (frame.width(), frame.height());
    let (w, h) = (DETECT_W.min(fw), DETECT_H.min(fh));
    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..h {
        let sy = (y * fh / h.max(1)).min(fh.saturating_sub(1));
        for x in 0..w {
            let sx = (x * fw / w.max(1)).min(fw.saturating_sub(1));
            let px = match &frame.pixels {
                Pixels::Rgba(img) => img.pixels[sy * fw + sx].to_array(),
                Pixels::Yuyv(b) => {
                    // The pixel pair's shared chroma, this pixel's luma.
                    let q = (sy * fw + (sx & !1)) * 2;
                    let luma = if sx & 1 == 0 { b[q] } else { b[q + 2] };
                    let [r, g, bl] = crate::yuv::ycbcr_to_rgb(luma, b[q + 1], b[q + 3]);
                    [r, g, bl, 255]
                }
            };
            let o = (y * w + x) * 4;
            rgba[o..o + 4].copy_from_slice(&px);
        }
    }
    Image { w, h, rgba }
}

/// `referenceAtSize`: a reference at another size, nearest neighbour.
pub fn scaled(img: &Image, w: usize, h: usize) -> Image {
    if img.w == w && img.h == h {
        return img.clone();
    }
    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..h {
        let sy = (y * img.h / h).min(img.h - 1);
        for x in 0..w {
            let sx = (x * img.w / w).min(img.w - 1);
            let (s, d) = ((sy * img.w + sx) * 4, (y * w + x) * 4);
            rgba[d..d + 4].copy_from_slice(&img.rgba[s..s + 4]);
        }
    }
    Image { w, h, rgba }
}

// ------------------------------------------------------------------ storage

fn decode_data_url(url: &str) -> Option<Image> {
    let b64 = url.strip_prefix("data:image/png;base64,")?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let (w, h) = (img.width() as usize, img.height() as usize);
    let img = Image { w, h, rgba: img.into_raw() };
    // Electron downscales anything larger on load; so does this.
    Some(if w > DETECT_W || h > DETECT_H { scaled(&img, DETECT_W.min(w), DETECT_H.min(h)) } else { img })
}

pub fn encode_data_url(img: &Image) -> String {
    let mut png = Vec::new();
    let buf = image::RgbaImage::from_raw(img.w as u32, img.h as u32, img.rgba.clone()).expect("size matches");
    let _ = buf.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png);
    format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png))
}

/// Electron's `noSignalReferences` value, decoded. Accepts the old
/// one-reference shape (`{dataUrl, ...}`) as `deserializeReferences` does.
pub fn load(value: Option<&Value>) -> HashMap<String, Arc<Vec<Image>>> {
    let mut out = HashMap::new();
    let Some(Value::Object(map)) = value else { return out };
    for (device, info) in map {
        let entries: Vec<&Value> = match info.get("references") {
            Some(Value::Array(list)) => list.iter().collect(),
            _ if info.get("dataUrl").is_some() => vec![info],
            _ => Vec::new(),
        };
        let decoded: Vec<Image> = entries.iter()
            .filter_map(|e| e.get("dataUrl").and_then(Value::as_str))
            .filter_map(|u| {
                let img = decode_data_url(u);
                if img.is_none() {
                    log_warn!("[Detection] a reference for {device} did not decode; skipped");
                }
                img
            })
            .collect();
        if !decoded.is_empty() {
            log_info!("[Detection] {} reference(s) for {device}", decoded.len());
            out.insert(device.clone(), Arc::new(decoded));
        }
    }
    out
}

/// Add one reference for a device, in Electron's shape.
pub fn append(value: &mut Option<Value>, device: &str, img: &Image) {
    let root = value.get_or_insert_with(|| Value::Object(Map::new()));
    if !root.is_object() {
        *root = Value::Object(Map::new());
    }
    let entry = root.as_object_mut().expect("object").entry(device.to_owned()).or_insert_with(|| json!({ "references": [] }));
    if !entry.get("references").is_some_and(Value::is_array) {
        *entry = json!({ "references": [] });
    }
    entry["references"].as_array_mut().expect("array")
        .push(json!({ "dataUrl": encode_data_url(img), "width": img.w, "height": img.h }));
}

pub fn clear(value: &mut Option<Value>, device: &str) {
    if let Some(Value::Object(map)) = value {
        map.remove(device);
    }
}

// ------------------------------------------------------------------- worker

struct Job {
    device: String,
    frame: Arc<Frame>,
    references: Arc<Vec<Image>>,
}

/// The comparison thread.
pub struct Detector {
    jobs: mpsc::Sender<Job>,
    results: mpsc::Receiver<(String, bool)>,
    in_flight: HashSet<String>,
}

impl Detector {
    pub fn new(repaint: impl Fn() + Send + 'static) -> Self {
        let (tx, jobs) = mpsc::channel::<Job>();
        let (done, results) = mpsc::channel();
        let _ = std::thread::Builder::new().name("no-signal-detect".into()).spawn(move || {
            for job in jobs {
                let frame = thumbnail(&job.frame);
                drop(job.frame);
                let refs: Vec<Image> = job.references.iter().map(|r| scaled(r, frame.w, frame.h)).collect();
                if done.send((job.device, matches_any(&frame, &refs))).is_err() {
                    break;
                }
                repaint();
            }
        });
        Self { jobs: tx, results, in_flight: HashSet::new() }
    }

    /// Queue a comparison unless one for this device is still running.
    pub fn submit(&mut self, device: &str, frame: Arc<Frame>, references: Arc<Vec<Image>>) {
        if self.in_flight.insert(device.to_owned()) {
            let _ = self.jobs.send(Job { device: device.to_owned(), frame, references });
        }
    }

    pub fn results(&mut self) -> Vec<(String, bool)> {
        let out: Vec<_> = self.results.try_iter().collect();
        for (d, _) in &out {
            self.in_flight.remove(d);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    //! Ported from test/detection.test.js where the logic exists here.
    use super::*;

    fn solid(w: usize, h: usize, rgb: [u8; 3]) -> Image {
        Image { w, h, rgba: (0..w * h).flat_map(|_| [rgb[0], rgb[1], rgb[2], 255]).collect() }
    }

    #[test]
    fn identical_frames_match_and_different_ones_do_not() {
        let a = solid(48, 27, [20, 20, 200]);
        assert!(compare(&a, &a.clone()));
        assert!(!compare(&a, &solid(48, 27, [200, 20, 20])));
    }

    #[test]
    fn the_per_channel_threshold_is_thirty() {
        let a = solid(48, 27, [100, 100, 100]);
        assert!(compare(&a, &solid(48, 27, [130, 70, 100])));
        assert!(!compare(&a, &solid(48, 27, [131, 100, 100])));
    }

    #[test]
    fn ninety_five_percent_of_samples_must_match() {
        let (w, h) = (100, 10);
        let reference = solid(w, h, [0, 0, 0]);
        let with_misses = |n: usize| {
            let mut f = reference.clone();
            // Sampled pixels are every 4th; spoil n of the 250 samples.
            for s in 0..n {
                let o = s * 4 * SAMPLE_RATE;
                f.rgba[o] = 255;
            }
            f
        };
        assert!(compare(&with_misses(12), &reference)); // 238/250 = 95.2%
        assert!(!compare(&with_misses(13), &reference)); // 237/250 = 94.8%
    }

    #[test]
    fn different_sizes_never_match() {
        assert!(!compare(&solid(48, 27, [0, 0, 0]), &solid(47, 27, [0, 0, 0])));
    }

    #[test]
    fn the_probe_never_rejects_what_the_scan_would_match() {
        // A frame at the edge of matching: 5% of pixels spoiled, spread out.
        let reference = solid(480, 270, [10, 10, 10]);
        let mut f = reference.clone();
        for p in (0..480 * 270).step_by(20) {
            f.rgba[p * 4] = 250;
        }
        if compare(&f, &reference) {
            assert!(probe(&f, &reference));
        }
    }

    #[test]
    fn any_reference_matching_is_no_signal() {
        let frame = solid(48, 27, [0, 0, 255]);
        assert!(matches_any(&frame, &[solid(48, 27, [0, 0, 0]), solid(48, 27, [0, 0, 250])]));
        assert!(!matches_any(&frame, &[]));
    }

    #[test]
    fn two_agreeing_results_flip_the_state() {
        let mut d = Debounce::default();
        assert!(!d.feed(true));
        assert!(d.feed(true));
        assert!(d.no_signal);
        // One live frame among no-signal ones changes nothing.
        assert!(!d.feed(false));
        assert!(!d.feed(true));
        assert!(!d.feed(false));
        assert!(d.feed(false));
        assert!(!d.no_signal);
    }

    #[test]
    fn references_round_trip_through_electrons_shape() {
        let img = solid(32, 18, [30, 128, 237]);
        let mut value = None;
        append(&mut value, "dev-a", &img);
        append(&mut value, "dev-a", &solid(32, 18, [0, 0, 0]));
        let v = value.as_ref().unwrap();
        assert_eq!(v["dev-a"]["references"][0]["width"], 32);
        assert!(v["dev-a"]["references"][0]["dataUrl"].as_str().unwrap().starts_with("data:image/png;base64,"));
        let loaded = load(value.as_ref());
        assert_eq!(loaded["dev-a"].len(), 2);
        assert_eq!(loaded["dev-a"][0], img);
        clear(&mut value, "dev-a");
        assert!(load(value.as_ref()).is_empty());
    }

    #[test]
    fn the_old_single_reference_shape_still_loads() {
        let url = encode_data_url(&solid(8, 8, [1, 2, 3]));
        let v = json!({ "dev": { "dataUrl": url, "width": 8, "height": 8 } });
        assert_eq!(load(Some(&v))["dev"].len(), 1);
    }

    #[test]
    fn a_yuyv_frame_thumbnails_to_its_colours() {
        // Mid-grey limited range: Y 126, neutral chroma.
        let (w, h) = (64, 36);
        let bytes: Vec<u8> = (0..w * h / 2).flat_map(|_| [126, 128, 126, 128]).collect();
        let frame = Frame::new_yuyv(w, h, bytes::Bytes::from(bytes), 1);
        let t = thumbnail(&frame);
        assert_eq!((t.w, t.h), (64, 36));
        assert!(t.rgba.chunks(4).all(|p| (i16::from(p[0]) - 128).abs() <= 1 && p[0] == p[1] && p[1] == p[2]));
    }
}
