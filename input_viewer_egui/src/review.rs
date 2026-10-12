// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Headless review hook: render, save a PNG of the window, quit.
//!
//! `screencapture` needs Screen Recording permission, which a CI runner or an
//! agent's shell does not have. egui can read back its own framebuffer, so this
//! captures exactly what was drawn without asking the OS.
//!
//! ```sh
//! INPUT_VIEWER_SCREENSHOT=out.png INPUT_VIEWER_OPEN=dropdown input-viewer --mock
//! ```
//!
//! `INPUT_VIEWER_OPEN` (`dropdown`, `legend`, `settings`, `settings:layout`) pins a panel open,
//! since there is no pointer to hover with. `INPUT_VIEWER_SCREENSHOT_AFTER` is
//! the delay in ms (default 2500).

use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui;

pub struct Review {
    path: PathBuf,
    due: Instant,
    requested: bool,
    pub open: Option<String>,
}

impl Review {
    pub fn from_env() -> Option<Self> {
        let path = std::env::var_os("INPUT_VIEWER_SCREENSHOT")?.into();
        let after = std::env::var("INPUT_VIEWER_SCREENSHOT_AFTER").ok().and_then(|v| v.parse().ok()).unwrap_or(2500);
        Some(Self {
            path,
            due: Instant::now() + Duration::from_millis(after),
            requested: false,
            open: std::env::var("INPUT_VIEWER_OPEN").ok(),
        })
    }

    pub fn tick(&mut self, ctx: &egui::Context) {
        if !self.requested && Instant::now() >= self.due {
            self.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = shot {
            let [w, h] = img.size;
            let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
            match image::save_buffer(&self.path, &bytes, w as u32, h as u32, image::ExtendedColorType::Rgba8) {
                Ok(()) => eprintln!("[review] saved {} ({w}x{h})", self.path.display()),
                Err(e) => eprintln!("[review] could not save {}: {e}", self.path.display()),
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
