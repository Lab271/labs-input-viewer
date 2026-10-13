// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Input Viewer, egui prototype.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// First, so its macros are in scope everywhere below.
mod filelog;

mod app;
mod capture;
mod crop;
mod flags;
mod gpu_video;
mod health;
mod logo;
mod remote_key;
mod render_stats;
mod review;
mod screensaver;
mod settings;
mod shader_saver;
mod settings_status;
mod shortcuts;
mod testpattern;
mod transition;
mod update;
mod yuv;

use crate::filelog::{log_error, log_info};

/// The log file, the startup banner and the panic hook (ADR 0044).
fn start_log() {
    if let Some(config) = dirs::config_dir() {
        filelog::init(config.join(settings::APP_DIR).join("logs"));
    }
    // OS uptime is here because the question after a bad boot is usually
    // "how long after the machine came up did the app open the cards".
    log_info!(
        "[App] Input Viewer (egui) {} starting; {} {}, OS up {}s, pid {}, log {}",
        env!("CARGO_PKG_VERSION"),
        sysinfo::System::long_os_version().unwrap_or_else(|| std::env::consts::OS.to_owned()),
        std::env::consts::ARCH,
        sysinfo::System::uptime(),
        std::process::id(),
        filelog::dir().map_or_else(|| "(none)".to_owned(), |d| d.display().to_string()),
    );
    // A panic is the one exit that would otherwise leave no trace in the file.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log_error!("[App] panic: {info}");
        default_hook(info);
    }));
}

fn main() -> eframe::Result {
    // Must run first: when the installer or an update launches the app with
    // its hook arguments, this handles them and exits before any window opens.
    velopack::VelopackApp::build().run();

    start_log();
    let flags = flags::parse(std::env::args().skip(1));
    for arg in &flags.unrecognised {
        log_error!("[TestFlags] unrecognised argument: {arg}");
    }
    if flags.mock.is_some() || flags.no_signal || flags.screensaver_delay != flags::DEFAULT_SCREENSAVER_DELAY {
        log_info!("[TestFlags] test mode: {flags:?}");
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
