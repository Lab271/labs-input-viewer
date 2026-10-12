// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! List capture devices and try to pull one frame from each.
//!
//! `cargo run --example camera_probe`
//!
//! Run it from your own terminal: macOS attributes camera permission to the
//! app that launched the process, and only a terminal app can show the prompt.

use std::sync::mpsc;
use std::time::Duration;

use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::{ApiBackend, RequestedFormat, RequestedFormatType};

fn main() {
    let (tx, rx) = mpsc::channel();
    nokhwa::nokhwa_initialize(move |granted| {
        let _ = tx.send(granted);
    });
    let granted = rx.recv_timeout(Duration::from_secs(60)).unwrap_or(false);
    println!("camera access granted: {granted}");

    for info in nokhwa::query(ApiBackend::Auto).unwrap_or_default() {
        print!("{} [{}]: ", info.human_name(), info.index());
        let fmt = RequestedFormat::new::<RgbAFormat>(RequestedFormatType::AbsoluteHighestResolution);
        let result = nokhwa::Camera::new(info.index().clone(), fmt).and_then(|mut cam| {
            cam.open_stream()?;
            let frame = cam.frame()?;
            let img = frame.decode_image::<RgbAFormat>()?;
            let _ = cam.stop_stream();
            Ok((cam.camera_format(), img.width(), img.height()))
        });
        match result {
            Ok((f, w, h)) => println!("OK {f} -> {w}x{h}"),
            Err(e) => println!("FAILED {e}"),
        }
    }
}
