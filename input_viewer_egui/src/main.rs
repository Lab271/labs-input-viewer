// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Input Viewer, egui prototype.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod capture;
mod flags;
mod health;
mod plasma;
mod render_stats;
mod review;
mod screensaver;
mod settings;
mod settings_status;
mod shortcuts;
mod testpattern;
mod update;

fn main() -> eframe::Result {
    // Must run first: when the installer or an update launches the app with
    // its hook arguments, this handles them and exits before any window opens.
    velopack::VelopackApp::build().run();

    let flags = flags::parse(std::env::args().skip(1));
    for arg in &flags.unrecognised {
        eprintln!("[flags] unrecognised argument: {arg}");
    }
    if flags.mock.is_some() || flags.no_signal || flags.screensaver_delay != flags::DEFAULT_SCREENSAVER_DELAY {
        eprintln!("[flags] test mode: {flags:?}");
    }

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Input Viewer")
            .with_inner_size([1500.0, 500.0])
            .with_min_inner_size([480.0, 160.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Input Viewer",
        options,
        Box::new(move |cc| Ok(Box::new(app::InputViewer::new(cc, flags)))),
    )
}
