// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! The Art-Net client: artnet-sync.js `createArtnetSync`, as a synchronous
//! state machine over a [`Transport`].
//!
//! Electron's client was async, with an `inFlight` flag so requests never
//! overlapped. Here the client lives on one worker thread (`worker.rs`) and
//! each request blocks it, so requests cannot overlap by construction and
//! the flag is gone. Everything else -- the rate limit, the backoff, the
//! effect bookkeeping, the snapshot -- is the JS logic statement for
//! statement, and the tests are its tests with the fake clock passed in.
//!
//! Two invariants worth more than the rest, both because this drives
//! physical fixtures in a room that may have people in it:
//! - nothing is sent unless enabled AND given a URL;
//! - release never blacks the room out: it puts back what was there
//!   (the snapshot), else the configured release scene, else nothing.

use std::collections::HashMap;

use chrono::{DateTime, Local};
use serde_json::{Map, Value, json};

use super::colour::{Dominant, GRID, Rgb, dominant_colour, luminance_focus, round4, round_u8};
use super::effects::effect_params;
use super::spatial::{self, Extent, Pixels, SPATIAL_CHANGE_THRESHOLD, SPATIAL_MAX_WRITES, SPATIAL_TRANSITION_MS, Strip};
use super::target::{self as t, Mode, Snapshot, effect_name_for, mode_for_saver, summarise_state};
use super::Config;
use crate::filelog::log_warn;

/// At most one send per second. The relay fades between values, so faster
/// buys nothing visible and adds readbacks and requests.
pub const SEND_INTERVAL_MS: u64 = 1000;
/// Fade asked of the relay: longer than the send interval, so consecutive
/// sends glide rather than step.
pub const TRANSITION_MS: u64 = 1400;
/// An unreachable relay is retried at 5 s, doubling to 5 minutes, so a wall
/// left running against a dead service does not retry every second for days.
const BACKOFF_MIN_MS: u64 = 5000;
const BACKOFF_MAX_MS: u64 = 5 * 60 * 1000;

/// How requests leave. A trait so the tests can stand in for the relay.
pub trait Transport {
    /// POST a JSON body. Err on transport failure or a non-2xx status.
    fn post(&mut self, url: &str, body: &Value) -> Result<(), String>;
    /// GET and parse JSON. A body that does not parse is an Err: restoring
    /// from an empty snapshot would leave the room lit.
    fn get(&mut self, url: &str) -> Result<Value, String>;
}

/// The relay's scene and effect names, for the per-saver dropdowns.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Catalogue {
    pub scenes: Vec<String>,
    pub effects: Vec<String>,
}

/// artnet-sync.js `getStatus()`, plus when and where the last send went,
/// for the Settings status line.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Status {
    pub sent: u64,
    pub failures: u32,
    pub last_error: Option<String>,
    pub last_colour: Option<Dominant>,
    pub last_spot: Option<Map<String, Value>>,
    pub running_effect: Option<String>,
    pub active_saver: Option<String>,
    pub active_mode: &'static str,
    /// Strips written in the last spatial tick, and columns sampled.
    pub last_spatial: Option<(usize, usize)>,
    pub has_prior_state: bool,
    pub next_eligible_at: u64,
    /// The last request: when, where, and whether it landed.
    pub last_request: Option<(DateTime<Local>, String, bool)>,
}

#[cfg(test)]
impl Status {
    pub fn effect_running(&self) -> bool {
        self.running_effect.is_some()
    }
}

struct Layout {
    strips: Vec<Strip>,
    extent: Extent,
}

pub struct Client<T: Transport> {
    transport: T,
    cfg: Config,
    last_sent_at: Option<u64>,
    failures: u32,
    last_error: Option<String>,
    sent: u64,
    last_colour: Option<Dominant>,
    last_spot: Option<Map<String, Value>>,
    last_spatial: Option<(usize, usize)>,
    last_request: Option<(DateTime<Local>, String, bool)>,
    /// The room before we took it over.
    prior_state: Option<Snapshot>,
    active_saver: Option<String>,
    active_mode: Mode,
    /// The effect believed running on the relay. A name, not a flag: a
    /// per-saver `effect:aurora` replacing a global `effect:spot` needs a
    /// fresh start, and a flag would say "already running" and nudge a field
    /// aurora never receives.
    running_effect: Option<String>,
    /// Spatial: the rig's geometry, fetched once per activation -- it is a
    /// property of the room, not of the frame.
    layout: Option<Layout>,
    layout_failed: bool,
    last_pixels: HashMap<String, Pixels>,
}

impl<T: Transport> Client<T> {
    pub fn new(transport: T, cfg: Config) -> Self {
        Self {
            transport,
            cfg: cfg.normalised(),
            last_sent_at: None,
            failures: 0,
            last_error: None,
            sent: 0,
            last_colour: None,
            last_spot: None,
            last_spatial: None,
            last_request: None,
            prior_state: None,
            active_saver: None,
            active_mode: Mode::Reactive,
            running_effect: None,
            layout: None,
            layout_failed: false,
            last_pixels: HashMap::new(),
        }
    }

    /// Read per call in Electron, so a toggle takes effect without a restart;
    /// here the worker hands every change in.
    pub fn set_config(&mut self, cfg: Config) {
        self.cfg = cfg.normalised();
    }

    #[cfg(test)]
    pub fn transport(&mut self) -> &mut T {
        &mut self.transport
    }

    fn live(&self) -> bool {
        self.cfg.enabled && !self.cfg.url.is_empty()
    }

    fn backoff_until(&self) -> u64 {
        if self.failures == 0 {
            return 0;
        }
        let shift = (self.failures - 1).min(20);
        let grown = BACKOFF_MIN_MS.saturating_mul(1 << shift);
        self.last_sent_at.unwrap_or(0) + grown.min(BACKOFF_MAX_MS)
    }

    /// Never fails outward; returns None and records the error. Unlike a
    /// POST it does not count towards the backoff, as in Electron.
    fn get(&mut self, url: &str) -> Option<Value> {
        match self.transport.get(url) {
            Ok(v) => Some(v),
            Err(e) => {
                log_warn!("[Art-Net] read failed: {e}");
                self.last_error = Some(e);
                None
            }
        }
    }

    /// Never fails outward; records the failure and backs off. A dark room
    /// is not worth a broken wall.
    fn post(&mut self, url: &str, body: &Value) -> bool {
        let res = self.transport.post(url, body);
        self.last_request = Some((Local::now(), url.to_owned(), res.is_ok()));
        match res {
            Ok(()) => {
                self.failures = 0;
                self.last_error = None;
                true
            }
            Err(e) => {
                self.failures += 1;
                log_warn!("[Art-Net] send failed ({}x): {e}", self.failures);
                self.last_error = Some(e);
                false
            }
        }
    }

    /// Offer a sampled frame (the sample grid's layout). Cheap when there is
    /// nothing to do: the enabled check, the mode, the 1 Hz cap and the
    /// backoff all gate here, so the caller needs to know none of them.
    pub fn offer_frame(&mut self, rgba: &[u8], now: u64) {
        if !self.live() {
            return;
        }
        // A saver on a scene or off is not driven from the picture. Frames
        // keep coming either way; this gate is what makes a scene hold.
        if matches!(self.active_mode, Mode::Off | Mode::Scene(_)) {
            return;
        }
        if self.last_sent_at.is_some_and(|at| now.saturating_sub(at) < SEND_INTERVAL_MS) {
            return;
        }
        if self.failures > 0 && now < self.backoff_until() {
            return;
        }

        let col = dominant_colour(rgba);
        let brightness = (col.brightness * self.cfg.max_brightness).clamp(0.0, 1.0);
        self.last_colour = Some(Dominant { brightness, ..col });
        self.last_sent_at = Some(now);
        self.sent += 1;

        // The global target can select spatial too, for installs that want it
        // everywhere rather than per saver.
        if self.active_mode == Mode::Spatial || (self.active_mode == Mode::Reactive && self.cfg.target == "spatial") {
            self.send_spatial(rgba);
            return;
        }

        let effect = match &self.active_mode {
            Mode::Effect(name) => Some(name.clone()),
            _ => effect_name_for(&self.cfg.target).map(str::to_owned),
        };
        let url = self.cfg.url.clone();
        if let Some(effect) = effect {
            let focus = luminance_focus(rgba, GRID);
            // Colour PRE-MULTIPLIED by brightness, not sent as its own key:
            // most effects take no brightness, the relay would drop the key
            // silently and maxBrightness would quietly stop capping. The two
            // that do take it get it from `level`.
            let scale = |c: u8| round_u8(f64::from(c) * brightness);
            let scaled = Rgb { r: scale(col.rgb.r), g: scale(col.rgb.g), b: scale(col.rgb.b) };
            let level = round4(brightness);
            let starting = self.running_effect.as_deref() != Some(effect.as_str());

            let Some(params) = effect_params(&effect, scaled, focus, level, self.cfg.spot_depth) else {
                // Not one we drive: started once and left alone.
                if !starting {
                    return;
                }
                self.last_spot = Some(Map::from_iter([("effect".into(), json!(effect)), ("params".into(), Value::Null)]));
                let ok = self.post(&t::effect_endpoint(&url, &effect), &json!({}));
                self.running_effect = ok.then_some(effect);
                return;
            };
            let mut spot = Map::from_iter([("effect".to_owned(), json!(effect))]);
            spot.extend(params.clone());
            self.last_spot = Some(spot);

            // The first send starts the effect (a flat EffectRequest); every
            // send after is a partial /field/params nudge -- restarting each
            // second would reset its animation phase and read as a stutter.
            let ok = if starting {
                self.post(&t::effect_endpoint(&url, &effect), &Value::Object(params))
            } else {
                self.post(&t::field_params_endpoint(&url), &json!({ "params": params }))
            };
            self.running_effect = ok.then_some(effect);
            return;
        }

        let Rgb { r, g, b } = col.rgb;
        self.post(&t::colour_endpoint(&url, &self.cfg.target), &json!({
            "r": r, "g": g, "b": b, "brightness": brightness, "transition_ms": TRANSITION_MS,
        }));
    }

    /// This frame's colours on the strips nearest them; sequential and
    /// capped by the plan, because a burst of forty POSTs at a relay running
    /// a DMX loop buys nothing when the next frame is a second away.
    fn send_spatial(&mut self, rgba: &[u8]) {
        let url = self.cfg.url.clone();
        if self.layout.is_none() {
            // One attempt per activation: retrying a missing layout every
            // second would be a request a second forever.
            if self.layout_failed {
                return;
            }
            let body = self.get(&t::layout_endpoint(&url));
            let strips: Option<Vec<Strip>> = body.as_ref()
                .and_then(|b| b.get("strips")?.as_array().cloned())
                .map(|a| a.iter().filter_map(Strip::from_value).collect());
            match strips.and_then(|s| Some((spatial::strip_extent(&s)?, s))) {
                Some((extent, strips)) => self.layout = Some(Layout { strips, extent }),
                None => {
                    self.layout_failed = true;
                    return;
                }
            }
        }
        let layout = self.layout.as_ref().expect("set above");
        let profile = spatial::column_profile(rgba, GRID);
        let plan = spatial::spatial_plan(&layout.strips, &profile, layout.extent, &self.last_pixels,
            SPATIAL_MAX_WRITES, SPATIAL_CHANGE_THRESHOLD);
        self.last_spatial = Some((plan.len(), profile.len()));
        for item in plan {
            let ok = self.post(&t::strip_pixels_endpoint(&url, &item.name), &json!({
                "pixels": item.pixels, "brightness": self.cfg.max_brightness, "transition_ms": SPATIAL_TRANSITION_MS,
            }));
            // Only remember what landed, so a failed write is retried rather
            // than taken as the strip's colour forever. And stop at the first
            // failure rather than hammer a relay that is refusing.
            if !ok {
                break;
            }
            self.last_pixels.insert(item.name, item.pixels);
        }
    }

    /// Not simply "mode is effect": a reactive saver inherits the global
    /// target, which may itself be an `effect:`.
    fn wants_effect(&self) -> bool {
        match &self.active_mode {
            Mode::Effect(_) => true,
            Mode::Reactive => effect_name_for(&self.cfg.target).is_some(),
            _ => false,
        }
    }

    /// Reactive modes do nothing here: they are driven from frames, and an
    /// effect starts on the first frame so it begins with the picture's
    /// colour rather than the relay's defaults.
    fn apply_mode(&mut self) {
        if let Mode::Scene(name) = &self.active_mode {
            let url = t::scene_endpoint(&self.cfg.url, name);
            self.post(&url, &json!({}));
        }
    }

    /// Take the room over for a saver, remembering what it looked like.
    /// Idempotent per activation: a second call would snapshot our OWN
    /// lighting and lose the original for good.
    pub fn activate(&mut self, saver: &str) {
        self.active_saver = Some(saver.to_owned());
        self.active_mode = mode_for_saver(saver, &self.cfg.scene_by_saver);
        if !self.live() {
            return;
        }
        if self.prior_state.is_none() {
            let url = t::status_endpoint(&self.cfg.url);
            self.prior_state = self.get(&url).as_ref().and_then(summarise_state);
        }
        self.apply_mode();
    }

    /// Switch saver mid-activation, keeping the original snapshot. Treating a
    /// rotation as a fresh activation is the obvious mistake: "before" would
    /// become the previous saver's lighting.
    pub fn rotate(&mut self, saver: &str) {
        self.active_saver = Some(saver.to_owned());
        self.active_mode = mode_for_saver(saver, &self.cfg.scene_by_saver);
        if !self.live() {
            return;
        }
        // Gated on what is RUNNING, not on the previous mode: a reactive saver
        // with a global effect target has an effect running while its own
        // mode is reactive. Effect to effect needs no stop -- starting the
        // next replaces it, and a stop would darken the room for a beat.
        if self.running_effect.is_some() && !self.wants_effect() {
            self.running_effect = None;
            let url = t::stop_endpoint(&self.cfg.url);
            self.post(&url, &json!({}));
        }
        self.apply_mode();
    }

    /// Hand the room back: the snapshot, else the release scene, else
    /// nothing -- never a blackout. An effect we started is stopped first,
    /// or it would keep animating under the restored colours.
    pub fn release(&mut self) {
        let was_running = self.running_effect.take().is_some();
        let snapshot = self.prior_state.take();
        // Forget what was written, or the next activation would skip every
        // strip as unchanged and leave the room as it is.
        self.last_pixels.clear();
        self.active_saver = None;
        self.active_mode = Mode::Reactive;
        if !self.live() {
            return;
        }
        let url = self.cfg.url.clone();
        if was_running {
            self.post(&t::stop_endpoint(&url), &json!({}));
        }
        if let Some(snapshot) = snapshot {
            self.restore(&snapshot);
            return;
        }
        if !self.cfg.release_scene.is_empty() {
            let scene = t::scene_endpoint(&url, &self.cfg.release_scene);
            self.post(&scene, &json!({}));
        }
    }

    /// An effect that was running when we arrived is restarted by name
    /// (at the relay's defaults: /status does not report its parameters),
    /// because the room was moving when we found it. A non-uniform room is
    /// restored strip by strip, sequentially.
    fn restore(&mut self, snapshot: &Snapshot) {
        let url = self.cfg.url.clone();
        if let Some(effect) = &snapshot.effect {
            self.post(&t::effect_endpoint(&url, effect), &json!({}));
            return;
        }
        if let Some(([r, g, b], brightness)) = snapshot.uniform {
            self.post(&t::colour_endpoint(&url, "all"), &json!({
                "r": r, "g": g, "b": b, "brightness": brightness, "transition_ms": TRANSITION_MS,
            }));
            return;
        }
        for st in &snapshot.strips {
            let [r, g, b] = st.rgb;
            self.post(&t::strip_endpoint(&url, &st.name), &json!({
                "r": r, "g": g, "b": b, "brightness": st.brightness, "transition_ms": TRANSITION_MS,
            }));
        }
    }

    /// Scene names from `/status`, for the per-saver dropdowns. Site-specific
    /// (`warm_wit` exists on this install only), so never hardcoded. None on
    /// any failure; the panel then keeps whatever is configured.
    pub fn read_catalogue(&mut self) -> Option<Catalogue> {
        if !self.live() {
            return None;
        }
        let url = t::status_endpoint(&self.cfg.url);
        let status = self.get(&url)?;
        let scenes = status.get("scenes").and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect())
            .unwrap_or_default();
        // /status lists scenes but not effects; those are fixed in the
        // relay's code, so the driven ones are named here.
        let effects = super::effects::DRIVEN.iter().map(|s| (*s).to_owned()).collect();
        Some(Catalogue { scenes, effects })
    }

    pub fn status(&self) -> Status {
        Status {
            sent: self.sent,
            failures: self.failures,
            last_error: self.last_error.clone(),
            last_colour: self.last_colour,
            last_spot: self.last_spot.clone(),
            running_effect: self.running_effect.clone(),
            active_saver: self.active_saver.clone(),
            active_mode: self.active_mode.kind(),
            last_spatial: self.last_spatial,
            has_prior_state: self.prior_state.is_some(),
            next_eligible_at: (self.last_sent_at.unwrap_or(0) + SEND_INTERVAL_MS).max(self.backoff_until()),
            last_request: self.last_request.clone(),
        }
    }
}

#[cfg(test)]
mod tests;
