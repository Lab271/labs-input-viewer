// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! How fast do capture devices deliver, alone and together?
//!
//! ```sh
//! capture_rate --list
//! capture_rate --match "Game Capture" --seconds 20          # every match, one process
//! capture_rate --match "Game Capture" --nth 0 --seconds 20  # only the first match
//! ```
//!
//! Opens each chosen device the way the app does (highest resolution, the
//! patched nokhwa format choice), on its own thread, and reads frames as
//! fast as they come, doing nothing with them. Every 120 frames, and at the
//! end, it prints the rate and the time each read spent waiting.
//!
//! On the wall two Elgato cards gave ~31 fps each when the app opened both,
//! and 60 when it opened one (2026-10-13). Run two cards in one process,
//! then one card in each of two processes at the same time: if the separate
//! processes get 60 each, the limit is in-process (Media Foundation, COM,
//! nokhwa); if they also get ~30, it is the driver or the hardware.

use std::time::{Duration, Instant};

use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::{ApiBackend, RequestedFormat, RequestedFormatType};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let seconds: u64 = value("--seconds").and_then(|v| v.parse().ok()).unwrap_or(20);
    let pattern = value("--match").unwrap_or_default().to_lowercase();
    let nth: Option<usize> = value("--nth").and_then(|v| v.parse().ok());

    let (tx, rx) = std::sync::mpsc::channel();
    nokhwa::nokhwa_initialize(move |granted| {
        let _ = tx.send(granted);
    });
    let _ = rx.recv_timeout(Duration::from_secs(30));

    let all = nokhwa::query(ApiBackend::Auto).unwrap_or_default();
    if args.iter().any(|a| a == "--list") {
        for (i, info) in all.iter().enumerate() {
            println!("{i}: {} [{}]", info.human_name(), info.index());
        }
        return;
    }
    let matching: Vec<_> = all.into_iter().filter(|i| i.human_name().to_lowercase().contains(&pattern)).collect();
    let chosen: Vec<_> = match nth {
        Some(n) => matching.into_iter().nth(n).into_iter().collect(),
        None => matching,
    };
    if chosen.is_empty() {
        eprintln!("no device matches {pattern:?}");
        std::process::exit(2);
    }
    println!("pid {} reading {} device(s) for {seconds} s", std::process::id(), chosen.len());

    let threads: Vec<_> = chosen.into_iter().map(|info| {
        std::thread::spawn(move || {
            let name = format!("{} [{}]", info.human_name(), info.index());
            let fmt = RequestedFormat::new::<RgbAFormat>(RequestedFormatType::AbsoluteHighestResolution);
            let mut cam = match nokhwa::Camera::new(info.index().clone(), fmt) {
                Ok(c) => c,
                Err(e) => return format!("{name}: open failed: {e}"),
            };
            if let Err(e) = cam.open_stream() {
                return format!("{name}: stream failed: {e}");
            }
            let f = cam.camera_format();
            println!("{name}: {}x{} {:?} @ {}", f.resolution().width_x, f.resolution().height_y, f.format(), f.frame_rate());
            let start = Instant::now();
            let (mut frames, mut waited) = (0u32, Duration::ZERO);
            let (mut win_frames, mut win_start, mut win_wait) = (0u32, Instant::now(), Duration::ZERO);
            while start.elapsed() < Duration::from_secs(seconds) {
                let t = Instant::now();
                if let Err(e) = cam.frame() {
                    return format!("{name}: read failed after {frames} frames: {e}");
                }
                let d = t.elapsed();
                (frames, waited, win_frames, win_wait) = (frames + 1, waited + d, win_frames + 1, win_wait + d);
                if win_frames == 120 {
                    println!("{name}: {:.1} fps, {:.1} ms per read",
                        120.0 / win_start.elapsed().as_secs_f64(), win_wait.as_secs_f64() * 1000.0 / 120.0);
                    (win_frames, win_start, win_wait) = (0, Instant::now(), Duration::ZERO);
                }
            }
            let _ = cam.stop_stream();
            format!("{name}: TOTAL {:.1} fps over {:.0} s, {:.1} ms per read",
                f64::from(frames) / start.elapsed().as_secs_f64(), start.elapsed().as_secs_f64(),
                waited.as_secs_f64() * 1000.0 / f64::from(frames.max(1)))
        })
    }).collect();
    for t in threads {
        println!("{}", t.join().unwrap_or_else(|_| "thread panicked".into()));
    }
}
