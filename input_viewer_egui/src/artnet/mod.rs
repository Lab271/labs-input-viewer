// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Art-Net reactive lighting (ADR 0030, Electron issue #59): while a
//! screensaver is up, the room's fixtures follow it through the lab's
//! `artnet-relay` HTTP service. Screen and room idle together.
//!
//! OFF unless configured: this posts to a host on the LAN and physically
//! changes the lighting, so nothing happens until `artnetEnabled` is set and
//! a URL is given. There is no default URL -- "the URL is the entire
//! capability", and guessing one would be guessing about somebody's network.
//!
//! The port of `artnet-sync.js`, split by what can be tested without a GPU
//! or a network:
//!
//! | Module | What | Electron |
//! |---|---|---|
//! | `colour` | dominant colour, luminance focus, spot size | `dominantColour`, `luminanceFocus` |
//! | `target` | endpoints, target parsing, per-saver modes, `/status` snapshot | `colourEndpoint`, `modeForSaver`, `summariseState` |
//! | `effects` | per-effect parameters | `EFFECT_DRIVERS` |
//! | `spatial` | strip-by-strip colours | `columnProfile` ... `spatialPlan` |
//! | `client` | the state machine: rate limit, backoff, modes, snapshot | `createArtnetSync` |
//! | `worker` | the thread and the HTTP transport | main-process `artnet-send` |
//! | `observer` | the GL frame readback | gl-base.js `observeFrames` |
//!
//! What changed in the port: no CORS and no main-process hop -- one worker
//! thread does the HTTP -- and the readback is an asynchronous PBO read of a
//! blit, not 32 synchronous `readPixels` (see `observer.rs`).

pub mod client;
pub mod colour;
pub mod effects;
pub mod observer;
pub mod spatial;
pub mod target;
pub mod worker;

use serde_json::{Map, Value};

use crate::settings::Settings;
pub use colour::DEFAULT_SPOT_DEPTH;

/// main/index.js defaults. Electron's client fell back to maxBrightness 1
/// for an absent key while main and renderer both default to 0.8 (one of
/// the default sets ADR 0041 / open question E7 describes); here there is
/// one set, main's.
pub const DEFAULT_MAX_BRIGHTNESS: f64 = 0.8;

/// What the client reads from settings.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub enabled: bool,
    pub url: String,
    /// `all` | `spatial` | `group:<name>` | `strip:<name>` | `effect:<name>`.
    pub target: String,
    /// Posted on release when there is no snapshot; empty sends nothing.
    pub release_scene: String,
    /// Ceiling on how bright the room is driven, so a white saver cannot
    /// dazzle. 1 is no limit.
    pub max_brightness: f64,
    pub spot_depth: f64,
    pub scene_by_saver: Map<String, Value>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            url: String::new(),
            target: "all".into(),
            release_scene: String::new(),
            max_brightness: DEFAULT_MAX_BRIGHTNESS,
            spot_depth: DEFAULT_SPOT_DEPTH,
            scene_by_saver: Map::new(),
        }
    }
}

impl Config {
    pub fn from_settings(s: &Settings) -> Self {
        Self {
            enabled: s.artnet_enabled,
            url: s.artnet_url.trim().to_owned(),
            target: s.artnet_target.clone(),
            release_scene: s.artnet_release_scene.trim().to_owned(),
            max_brightness: s.artnet_max_brightness,
            spot_depth: s.artnet_spot_depth,
            scene_by_saver: s.artnet_scene_by_saver.clone(),
        }
        .normalised()
    }

    /// artnet-sync.js `config()`: an empty target is `all`, and the spot
    /// depth is clamped to the room -- the field is 0..1, and a cy of 5
    /// would place the spot where the relay's behaviour is not worth
    /// guessing at.
    pub fn normalised(mut self) -> Self {
        if self.target.trim().is_empty() {
            self.target = "all".into();
        }
        self.spot_depth = if self.spot_depth.is_finite() { self.spot_depth.clamp(0.0, 1.0) } else { DEFAULT_SPOT_DEPTH };
        if !self.max_brightness.is_finite() {
            self.max_brightness = DEFAULT_MAX_BRIGHTNESS;
        }
        self
    }
}
