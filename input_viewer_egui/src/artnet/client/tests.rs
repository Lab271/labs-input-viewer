// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! artnet-sync.test.js, artnet-spot.test.js, artnet-per-saver.test.js and
//! the client halves of artnet-spatial.test.js and
//! artnet-effect-params.test.js, case for case.
//!
//! The relay is a fake transport that records what it receives; the clock is
//! a number. Not ported, because the port makes them true by construction:
//! "does not overlap requests" (one worker thread, blocking requests) and
//! "leaves no timer pending" (the timeout lives inside the HTTP call).

use serde_json::{Value, json};

use super::*;
use crate::artnet::colour::tests::{grid, one_tile, solid};
use crate::artnet::colour::{DEFAULT_SPOT_DEPTH, GRID, SPOT_DIAMETER_MAX, SPOT_DIAMETER_MIN, SPOT_SOFTNESS};
use crate::artnet::target::{layout_endpoint, scene_endpoint, status_endpoint, strip_endpoint, strip_pixels_endpoint};

const BASE: &str = "http://pi.labs:8000";
const START: u64 = 1_700_000_000_000;

#[derive(Debug, Clone)]
struct Call {
    method: &'static str,
    url: String,
    body: Value,
}

type PostFn = Box<dyn FnMut(&str) -> Result<(), String>>;
type GetFn = Box<dyn FnMut(&str) -> Result<Value, String>>;

/// Stands in for the relay. Records every request, answers from closures.
struct Relay {
    calls: Vec<Call>,
    post: PostFn,
    get: GetFn,
}

impl Relay {
    fn ok() -> Self {
        Self { calls: Vec::new(), post: Box::new(|_| Ok(())), get: Box::new(|_| Ok(json!({ "ok": true }))) }
    }
    fn failing() -> Self {
        Self { post: Box::new(|_| Err("Failed to fetch".into())), get: Box::new(|_| Err("Failed to fetch".into())), ..Self::ok() }
    }
    /// GET /status answers with `status`.
    fn with_status(status: Value) -> Self {
        Self { get: Box::new(move |_| Ok(status.clone())), ..Self::ok() }
    }
    fn posts(&self) -> Vec<&Call> {
        self.calls.iter().filter(|c| c.method == "POST").collect()
    }
    fn post_urls(&self) -> Vec<String> {
        self.posts().iter().map(|c| c.url.clone()).collect()
    }
    fn gets(&self) -> usize {
        self.calls.iter().filter(|c| c.method == "GET").count()
    }
}

impl Transport for Relay {
    fn post(&mut self, url: &str, body: &Value) -> Result<(), String> {
        let r = (self.post)(url);
        // A thrown fetch never reaches the relay; an HTTP error did.
        if r.as_ref().is_ok_and(|()| true) || r.as_ref().is_err_and(|e| e.starts_with("HTTP")) {
            self.calls.push(Call { method: "POST", url: url.to_owned(), body: body.clone() });
        }
        r
    }
    fn get(&mut self, url: &str) -> Result<Value, String> {
        self.calls.push(Call { method: "GET", url: url.to_owned(), body: Value::Null });
        (self.get)(url)
    }
}

fn config() -> Config {
    Config { enabled: true, url: BASE.into(), max_brightness: 1.0, ..Config::default() }
}

fn with(f: impl FnOnce(&mut Config)) -> Config {
    let mut c = config();
    f(&mut c);
    c
}

fn by_saver(v: Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap()
}

/// A client, its fake relay, and a clock.
struct H {
    sync: Client<Relay>,
    now: u64,
}

impl H {
    fn new(relay: Relay, cfg: Config) -> Self {
        Self { sync: Client::new(relay, cfg), now: START }
    }
    fn relay(&mut self) -> &mut Relay {
        self.sync.transport()
    }
    fn offer(&mut self, frame: &[u8]) {
        self.sync.offer_frame(frame, self.now);
    }
    /// One send, then past the interval -- the JS tests' `send()`.
    fn send(&mut self, frame: &[u8]) {
        self.offer(frame);
        self.now += SEND_INTERVAL_MS;
    }
    fn clear(&mut self) {
        self.relay().calls.clear();
    }
    fn body(&mut self, i: usize) -> Value {
        self.relay().posts()[i].body.clone()
    }
}

fn f(v: &Value, k: &str) -> f64 {
    v[k].as_f64().unwrap_or_else(|| panic!("{k} missing in {v}"))
}

fn keys(v: &Value) -> Vec<String> {
    let mut k: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
    k.sort();
    k
}

fn close(a: f64, b: f64, digits: i32) -> bool {
    (a - b).abs() < 10f64.powi(-digits) / 2.0
}

const WHITE: [u8; 3] = [255, 255, 255];

/// A 4x8x8x8 frame of one colour: the per-saver and effect-params tests'.
fn frame() -> Vec<u8> {
    solid(200, 90, 30, 8 * 8 * 8 * 4)
}

// =================================================== artnet-sync.test.js

mod sending_nothing_unless_configured {
    use super::*;

    #[test]
    fn is_silent_when_disabled() {
        let mut h = H::new(Relay::ok(), with(|c| c.enabled = false));
        for _ in 0..100 {
            h.offer(&solid(255, 0, 0, 512));
            h.now += 16;
        }
        assert!(h.relay().calls.is_empty());
    }

    #[test]
    fn is_silent_with_no_url_even_when_enabled() {
        let mut h = H::new(Relay::ok(), with(|c| c.url = String::new()));
        for _ in 0..100 {
            h.offer(&solid(255, 0, 0, 512));
        }
        assert!(h.relay().calls.is_empty());
    }

    #[test]
    fn sends_nothing_on_release_unless_a_scene_is_configured() {
        let mut h = H::new(Relay::ok(), config());
        h.sync.release();
        assert!(h.relay().calls.is_empty());
    }

    #[test]
    fn posts_the_configured_scene_on_release() {
        let mut h = H::new(Relay::ok(), with(|c| c.release_scene = "warm_wit".into()));
        h.sync.release();
        assert_eq!(h.relay().post_urls(), [format!("{BASE}/scenes/warm_wit")]);
    }
}

mod what_reaches_the_relay {
    use super::*;

    #[test]
    fn matches_the_color_request_contract() {
        let mut h = H::new(Relay::ok(), config());
        h.offer(&solid(0, 200, 255, 512));
        assert_eq!(h.relay().calls.len(), 1);
        assert_eq!(h.relay().post_urls(), [format!("{BASE}/all")]);
        let body = h.body(0);
        // Exactly the fields lab271_artnet.py's ColorRequest declares.
        assert_eq!(keys(&body), ["b", "brightness", "g", "r", "transition_ms"]);
        for k in ["r", "g", "b"] {
            assert!(body[k].as_u64().is_some_and(|v| v <= 255), "{k}: {}", body[k]);
        }
        assert!((0.0..=1.0).contains(&f(&body, "brightness")));
        assert_eq!(body["transition_ms"], TRANSITION_MS);
    }

    #[test]
    fn caps_brightness_so_a_white_screensaver_cannot_dazzle() {
        let mut h = H::new(Relay::ok(), with(|c| c.max_brightness = 0.4));
        h.offer(&solid(255, 255, 255, 512));
        assert!(f(&h.body(0), "brightness") <= 0.4);
    }

    #[test]
    fn asks_for_a_fade_longer_than_the_send_interval() {
        const { assert!(TRANSITION_MS > SEND_INTERVAL_MS) };
    }
}

mod rate_limiting {
    use super::*;

    #[test]
    fn sends_at_most_once_per_interval_however_often_offered() {
        let mut h = H::new(Relay::ok(), config());
        for _ in 0..60 {
            h.offer(&solid(255, 0, 0, 512));
            h.now += 16;
        }
        assert!(h.relay().calls.len() <= 2);
    }

    #[test]
    fn sends_again_after_the_interval() {
        let mut h = H::new(Relay::ok(), config());
        h.offer(&solid(255, 0, 0, 512));
        assert_eq!(h.relay().calls.len(), 1);
        h.now += SEND_INTERVAL_MS + 1;
        h.offer(&solid(0, 255, 0, 512));
        assert_eq!(h.relay().calls.len(), 2);
    }
}

mod when_the_relay_is_unreachable {
    use super::*;

    #[test]
    fn counts_a_failure_and_carries_on() {
        let mut h = H::new(Relay::failing(), config());
        h.offer(&solid(255, 0, 0, 512));
        assert_eq!(h.sync.status().failures, 1);
    }

    #[test]
    fn backs_off_instead_of_retrying_every_second() {
        let tries = std::rc::Rc::new(std::cell::Cell::new(0));
        let t = tries.clone();
        let relay = Relay { post: Box::new(move |_| { t.set(t.get() + 1); Err("down".into()) }), ..Relay::ok() };
        let mut h = H::new(relay, config());
        h.offer(&solid(255, 0, 0, 512));
        assert_eq!(tries.get(), 1);
        h.now += SEND_INTERVAL_MS + 1;
        h.offer(&solid(255, 0, 0, 512));
        assert_eq!(tries.get(), 1, "held by the backoff");
        h.now += 10_000;
        h.offer(&solid(255, 0, 0, 512));
        assert_eq!(tries.get(), 2, "tries once more past the window");
    }

    #[test]
    fn treats_an_http_error_as_a_failure() {
        for status in [404, 500, 503] {
            let relay = Relay { post: Box::new(move |_| Err(format!("HTTP {status}"))), ..Relay::ok() };
            let mut h = H::new(relay, config());
            h.offer(&solid(255, 0, 0, 512));
            assert_eq!(h.sync.status().failures, 1, "status {status}");
        }
    }

    #[test]
    fn recovers_and_resets_the_backoff() {
        let failing = std::rc::Rc::new(std::cell::Cell::new(true));
        let fl = failing.clone();
        let relay = Relay { post: Box::new(move |_| if fl.get() { Err("down".into()) } else { Ok(()) }), ..Relay::ok() };
        let mut h = H::new(relay, config());
        h.offer(&solid(255, 0, 0, 512));
        assert_eq!(h.sync.status().failures, 1);
        failing.set(false);
        h.now += 10 * 60 * 1000;
        h.offer(&solid(255, 0, 0, 512));
        assert_eq!(h.sync.status().failures, 0);
    }

    /// Electron checked the status kept a fixed shape; a struct has one, so
    /// what is left to pin is that 200 doublings neither overflow nor push
    /// the next attempt past the 5-minute ceiling.
    #[test]
    fn a_long_outage_caps_the_backoff_at_five_minutes() {
        let mut h = H::new(Relay::failing(), config());
        for _ in 0..200 {
            h.now += 10 * 60 * 1000;
            h.offer(&solid(255, 0, 0, 512));
        }
        let s = h.sync.status();
        assert_eq!(s.failures, 200);
        assert_eq!(s.next_eligible_at, h.now + 5 * 60 * 1000);
    }
}

// =================================================== artnet-spot.test.js

fn spot() -> Config {
    with(|c| c.target = "effect:spot".into())
}

mod spot_mode_sends {
    use super::*;

    #[test]
    fn starts_the_effect_once_then_only_nudges_the_field() {
        let mut h = H::new(Relay::ok(), spot());
        h.send(&one_tile(1, 1, WHITE));
        h.send(&one_tile(6, 1, WHITE));
        h.send(&one_tile(4, 1, WHITE));
        assert_eq!(h.relay().post_urls(), [
            format!("{BASE}/effects/spot"), format!("{BASE}/field/params"), format!("{BASE}/field/params"),
        ]);
        let (start, nudge) = (h.body(0), h.body(1));
        assert!(close(f(&start, "cx"), 1.5 / GRID.tiles_x as f64, 4));
        assert!(start.get("params").is_none(), "start is a flat EffectRequest");
        assert!(close(f(&nudge["params"], "cx"), 6.5 / GRID.tiles_x as f64, 4));
    }

    #[test]
    fn moves_cx_with_the_picture_and_holds_cy_at_the_depth() {
        let mut h = H::new(Relay::ok(), with(|c| { c.target = "effect:spot".into(); c.spot_depth = 0.2; }));
        h.send(&one_tile(0, 0, WHITE));
        h.send(&one_tile(GRID.tiles_x - 1, GRID.tiles_y - 1, WHITE));
        let (start, nudge) = (h.body(0), h.body(1));
        assert!(close(f(&start, "cy"), 0.2, 6));
        assert!(close(f(&nudge["params"], "cy"), 0.2, 6), "a vertical move must not move cy");
        assert!(f(&nudge["params"], "cx") > f(&start, "cx"));
    }

    #[test]
    fn clamps_a_depth_outside_the_room_to_its_walls() {
        let mut far = H::new(Relay::ok(), with(|c| { c.target = "effect:spot".into(); c.spot_depth = 5.0; }));
        far.send(&one_tile(2, 2, WHITE));
        assert_eq!(f(&far.body(0), "cy"), 1.0);
        let mut near = H::new(Relay::ok(), with(|c| { c.target = "effect:spot".into(); c.spot_depth = -3.0; }));
        near.send(&one_tile(2, 2, WHITE));
        assert_eq!(f(&near.body(0), "cy"), 0.0);
    }

    #[test]
    fn defaults_depth_to_the_middle_of_the_room() {
        let mut h = H::new(Relay::ok(), with(|c| { c.target = "effect:spot".into(); c.spot_depth = f64::NAN; }));
        h.send(&one_tile(2, 2, WHITE));
        assert!(close(f(&h.body(0), "cy"), DEFAULT_SPOT_DEPTH, 6));
        assert_eq!(Config::default().spot_depth, DEFAULT_SPOT_DEPTH);
    }

    #[test]
    fn premultiplies_brightness_into_rgb_because_spot_has_no_brightness() {
        let mut full = H::new(Relay::ok(), spot());
        full.send(&grid(|_, _| WHITE));
        let mut dim = H::new(Relay::ok(), with(|c| { c.target = "effect:spot".into(); c.max_brightness = 0.25; }));
        dim.send(&grid(|_, _| WHITE));
        let (a, b) = (full.body(0), dim.body(0));
        assert!(f(&b, "r") < f(&a, "r"));
        assert!((f(&b, "r") / f(&a, "r") - 0.25).abs() < 0.05);
        assert!(a.get("brightness").is_none() && b.get("brightness").is_none());
    }

    #[test]
    fn sends_the_shape_the_effect_actually_takes() {
        let mut h = H::new(Relay::ok(), spot());
        h.send(&one_tile(3, 1, [255, 40, 40]));
        let body = h.body(0);
        assert_eq!(keys(&body), ["b", "cx", "cy", "diameter", "g", "r", "softness"]);
        assert!(close(f(&body, "softness"), SPOT_SOFTNESS, 6));
        for k in ["r", "g", "b"] {
            assert!(body[k].as_u64().is_some_and(|v| v <= 255));
        }
        assert!((SPOT_DIAMETER_MIN..=SPOT_DIAMETER_MAX).contains(&f(&body, "diameter")));
    }

    #[test]
    fn rounds_field_values_instead_of_shipping_float_noise() {
        // A uniform grid's centroid is 0.5000000000000001 and its diameter
        // 0.7951371798225177 before rounding.
        let mut h = H::new(Relay::ok(), spot());
        h.send(&grid(|_, _| [200, 90, 30]));
        let body = h.body(0);
        assert_eq!(f(&body, "cx"), 0.5);
        assert_eq!(f(&body, "diameter"), 0.7951);
        for k in ["cx", "cy", "diameter"] {
            let s = body[k].to_string();
            assert!(s.split('.').nth(1).unwrap_or("").len() <= 4, "{k} = {s}");
        }
    }

    #[test]
    fn restarts_the_effect_after_a_failed_send_rather_than_nudging_a_dead_field() {
        let ok = std::rc::Rc::new(std::cell::Cell::new(false));
        let o = ok.clone();
        let relay = Relay { post: Box::new(move |_| if o.get() { Ok(()) } else { Err("down".into()) }), ..Relay::ok() };
        let mut h = H::new(relay, spot());
        h.send(&one_tile(1, 1, WHITE));
        assert!(!h.sync.status().effect_running());
        ok.set(true);
        h.now += 10 * 60 * 1000;
        h.send(&one_tile(1, 1, WHITE));
        assert_eq!(h.relay().post_urls(), [format!("{BASE}/effects/spot")]);
    }

    #[test]
    fn leaves_colour_targets_on_the_flat_colour_path() {
        let mut h = H::new(Relay::ok(), config());
        h.send(&grid(|_, _| WHITE));
        assert_eq!(h.relay().post_urls(), [format!("{BASE}/all")]);
        let body = h.body(0);
        assert!(body.get("brightness").is_some() && body.get("transition_ms").is_some() && body.get("cx").is_none());
    }
}

mod spot_mode_release {
    use super::*;

    #[test]
    fn stops_an_effect_it_started_so_the_room_does_not_pulse_forever() {
        let mut h = H::new(Relay::ok(), spot());
        h.send(&one_tile(2, 1, WHITE));
        h.clear();
        h.sync.release();
        assert_eq!(h.relay().post_urls(), [format!("{BASE}/stop")]);
        assert!(!h.sync.status().effect_running());
    }

    #[test]
    fn sends_nothing_when_it_never_started_an_effect() {
        let mut h = H::new(Relay::ok(), spot());
        h.sync.release();
        assert!(h.relay().calls.is_empty());
    }

    #[test]
    fn stops_the_effect_before_posting_the_release_scene() {
        let mut h = H::new(Relay::ok(), with(|c| { c.target = "effect:spot".into(); c.release_scene = "warm_wit".into(); }));
        h.send(&one_tile(2, 1, WHITE));
        h.clear();
        h.sync.release();
        assert_eq!(h.relay().post_urls(), [format!("{BASE}/stop"), format!("{BASE}/scenes/warm_wit")]);
    }

    #[test]
    fn still_sends_nothing_at_all_when_disabled() {
        let mut h = H::new(Relay::ok(), with(|c| { c.target = "effect:spot".into(); c.enabled = false; }));
        h.send(&one_tile(2, 1, WHITE));
        h.sync.release();
        assert!(h.relay().calls.is_empty());
    }

    #[test]
    fn does_not_leave_the_effect_marked_running_after_release() {
        let mut h = H::new(Relay::ok(), spot());
        h.send(&one_tile(2, 1, WHITE));
        h.sync.release();
        h.clear();
        h.send(&one_tile(2, 1, WHITE));
        assert_eq!(h.relay().post_urls()[0], format!("{BASE}/effects/spot"));
    }
}

// =============================================== artnet-per-saver.test.js

fn uniform_status(rgb: [i64; 3], brightness: f64, effect: Value) -> Value {
    let strips: Vec<Value> = (1..=40).map(|i| json!({ "name": format!("u0_{i:02}"), "rgb": rgb, "brightness": brightness })).collect();
    json!({ "effect": effect, "strips": strips })
}

fn default_status() -> Value {
    uniform_status([40, 10, 80], 0.6, Value::Null)
}

fn mixed_status() -> Value {
    let mut s = default_status();
    s["strips"][7] = json!({ "name": "u0_08", "rgb": [255, 0, 0], "brightness": 1 });
    s
}

fn read_fails() -> Relay {
    Relay { get: Box::new(|_| Err("Failed to fetch".into())), ..Relay::ok() }
}

fn mapped(m: Value) -> Config {
    with(|c| c.scene_by_saver = by_saver(m))
}

mod activation {
    use super::*;

    #[test]
    fn reads_the_room_before_touching_it() {
        let mut h = H::new(Relay::with_status(default_status()), config());
        h.sync.activate("plasma");
        let first = h.relay().calls[0].clone();
        assert_eq!((first.method, first.url), ("GET", status_endpoint(BASE)));
        assert!(h.sync.status().has_prior_state);
    }

    #[test]
    fn posts_the_scene_for_a_saver_mapped_to_one() {
        let mut h = H::new(Relay::with_status(default_status()), mapped(json!({ "matrixRain": "scene:lab_modus" })));
        h.sync.activate("matrixRain");
        assert_eq!(h.relay().post_urls(), [scene_endpoint(BASE, "lab_modus")]);
    }

    #[test]
    fn sends_nothing_on_activation_for_a_reactive_saver() {
        let mut h = H::new(Relay::with_status(default_status()), config());
        h.sync.activate("plasma");
        assert!(h.relay().posts().is_empty());
    }

    #[test]
    fn does_not_re_snapshot_if_activate_is_called_twice() {
        let mut h = H::new(Relay::with_status(default_status()), config());
        h.sync.activate("a");
        h.sync.activate("a");
        h.sync.activate("b");
        assert_eq!(h.relay().gets(), 1);
    }

    #[test]
    fn does_not_read_or_write_when_disabled() {
        let mut h = H::new(Relay::with_status(default_status()), with(|c| c.enabled = false));
        h.sync.activate("plasma");
        assert!(h.relay().calls.is_empty());
    }
}

mod per_saver_modes_while_frames_arrive {
    use super::*;

    #[test]
    fn holds_a_scene_instead_of_letting_the_reactive_colour_overwrite_it() {
        let mut h = H::new(Relay::with_status(default_status()), mapped(json!({ "a": "scene:warm_wit" })));
        h.sync.activate("a");
        h.clear();
        for _ in 0..5 {
            h.send(&frame());
        }
        assert!(h.relay().posts().is_empty());
    }

    #[test]
    fn sends_nothing_at_all_for_a_saver_mapped_to_off() {
        let mut h = H::new(Relay::with_status(default_status()), mapped(json!({ "a": "off" })));
        h.sync.activate("a");
        h.clear();
        h.offer(&frame());
        assert!(h.relay().posts().is_empty());
    }

    #[test]
    fn lets_a_per_saver_effect_override_the_global_colour_target() {
        let mut h = H::new(Relay::with_status(default_status()), mapped(json!({ "a": "effect:aurora" })));
        h.sync.activate("a");
        h.clear();
        h.offer(&frame());
        assert_eq!(h.relay().post_urls()[0], format!("{BASE}/effects/aurora"));
    }

    #[test]
    fn still_drives_the_reactive_colour_for_an_unmapped_saver() {
        let mut h = H::new(Relay::with_status(default_status()), mapped(json!({ "other": "off" })));
        h.sync.activate("a");
        h.clear();
        h.offer(&frame());
        assert_eq!(h.relay().post_urls()[0], format!("{BASE}/all"));
    }
}

mod rotation {
    use super::*;

    #[test]
    fn does_not_re_snapshot_so_the_original_state_survives() {
        let mut h = H::new(Relay::with_status(default_status()), mapped(json!({ "b": "scene:warm_wit" })));
        h.sync.activate("a");
        assert_eq!(h.relay().gets(), 1);
        h.sync.rotate("b");
        h.sync.rotate("a");
        h.sync.rotate("b");
        assert_eq!(h.relay().gets(), 1);
    }

    #[test]
    fn restores_the_original_colour_after_rotating() {
        let mut h = H::new(Relay::with_status(default_status()), mapped(json!({ "b": "scene:warm_wit" })));
        h.sync.activate("a");
        h.sync.rotate("b");
        h.clear();
        h.sync.release();
        let restore = h.relay().posts().into_iter().find(|c| c.url == format!("{BASE}/all")).cloned().expect("an /all");
        assert_eq!((&restore.body["r"], &restore.body["g"], &restore.body["b"]), (&json!(40), &json!(10), &json!(80)));
        assert_eq!(f(&restore.body, "brightness"), 0.6);
    }

    #[test]
    fn does_not_post_a_spurious_stop_when_no_effect_was_running() {
        let mut h = H::new(Relay::with_status(default_status()), mapped(json!({ "b": "scene:warm_wit", "c": "scene:diep_blauw" })));
        h.sync.activate("b");
        h.clear();
        h.sync.rotate("c");
        h.sync.rotate("b");
        assert_eq!(h.relay().post_urls(), [scene_endpoint(BASE, "diep_blauw"), scene_endpoint(BASE, "warm_wit")]);
    }

    #[test]
    fn keeps_a_globally_targeted_effect_running_across_reactive_savers() {
        let mut h = H::new(Relay::with_status(default_status()), spot());
        h.sync.activate("a");
        h.send(&frame());
        h.clear();
        h.sync.rotate("b");
        h.sync.rotate("c");
        assert!(!h.relay().post_urls().contains(&format!("{BASE}/stop")));
        assert_eq!(h.sync.status().running_effect.as_deref(), Some("spot"));
        h.offer(&frame());
        assert_eq!(h.relay().post_urls()[0], format!("{BASE}/field/params"));
    }

    #[test]
    fn stops_a_globally_targeted_effect_when_rotating_to_a_scene_saver() {
        let mut h = H::new(Relay::with_status(default_status()),
            with(|c| { c.target = "effect:spot".into(); c.scene_by_saver = by_saver(json!({ "b": "scene:warm_wit" })); }));
        h.sync.activate("a");
        h.send(&frame());
        assert_eq!(h.sync.status().running_effect.as_deref(), Some("spot"));
        h.clear();
        h.sync.rotate("b");
        assert_eq!(h.relay().post_urls(), [format!("{BASE}/stop"), scene_endpoint(BASE, "warm_wit")]);
    }

    #[test]
    fn starts_the_new_effect_by_name_when_the_effect_changes() {
        let mut h = H::new(Relay::with_status(default_status()),
            with(|c| { c.target = "effect:spot".into(); c.scene_by_saver = by_saver(json!({ "b": "effect:aurora" })); }));
        h.sync.activate("a");
        h.send(&frame());
        h.sync.rotate("b");
        h.clear();
        h.offer(&frame());
        assert_eq!(h.relay().post_urls()[0], format!("{BASE}/effects/aurora"));
    }

    #[test]
    fn switches_effect_to_effect_directly_without_stopping_in_between() {
        let mut h = H::new(Relay::with_status(default_status()), mapped(json!({ "a": "effect:spot", "b": "effect:aurora" })));
        h.sync.activate("a");
        h.send(&frame());
        h.clear();
        h.sync.rotate("b");
        assert!(!h.relay().post_urls().contains(&format!("{BASE}/stop")));
        h.offer(&frame());
        assert!(h.relay().post_urls().contains(&format!("{BASE}/effects/aurora")));
    }

    #[test]
    fn stops_an_effect_when_rotating_to_a_saver_that_does_not_want_one() {
        let mut h = H::new(Relay::with_status(default_status()), mapped(json!({ "a": "effect:spot", "b": "scene:warm_wit" })));
        h.sync.activate("a");
        h.send(&frame());
        h.clear();
        h.sync.rotate("b");
        assert_eq!(h.relay().post_urls(), [format!("{BASE}/stop"), scene_endpoint(BASE, "warm_wit")]);
    }
}

mod release_restores_the_room {
    use super::*;

    #[test]
    fn puts_a_uniform_room_back_with_one_all() {
        let mut h = H::new(Relay::with_status(default_status()), config());
        h.sync.activate("a");
        h.clear();
        h.sync.release();
        assert_eq!(h.relay().post_urls(), [format!("{BASE}/all")]);
        let body = h.body(0);
        assert_eq!((&body["r"], &body["g"], &body["b"]), (&json!(40), &json!(10), &json!(80)));
        assert_eq!(f(&body, "brightness"), 0.6);
    }

    #[test]
    fn restores_lights_off_rather_than_leaving_the_room_lit() {
        let mut h = H::new(Relay::with_status(uniform_status([0, 0, 0], 1.0, Value::Null)), config());
        h.sync.activate("a");
        h.clear();
        h.sync.release();
        let body = h.body(0);
        assert_eq!((&body["r"], &body["g"], &body["b"]), (&json!(0), &json!(0), &json!(0)));
    }

    #[test]
    fn restores_a_mixed_room_strip_by_strip() {
        let mut h = H::new(Relay::with_status(mixed_status()), config());
        h.sync.activate("a");
        h.clear();
        h.sync.release();
        let urls = h.relay().post_urls();
        assert_eq!(urls.len(), 40);
        assert_eq!(urls[0], strip_endpoint(BASE, "u0_01"));
        let odd = h.relay().posts().into_iter().find(|c| c.url == strip_endpoint(BASE, "u0_08")).cloned().unwrap();
        assert_eq!((&odd.body["r"], &odd.body["g"], &odd.body["b"]), (&json!(255), &json!(0), &json!(0)));
    }

    #[test]
    fn restarts_an_effect_that_was_already_running_before_we_arrived() {
        let mut h = H::new(Relay::with_status(uniform_status([0, 0, 0], 1.0, json!("aurora"))), config());
        h.sync.activate("a");
        h.clear();
        h.sync.release();
        assert_eq!(h.relay().post_urls(), [format!("{BASE}/effects/aurora")]);
    }

    #[test]
    fn falls_back_to_the_release_scene_when_the_read_failed() {
        let mut h = H::new(read_fails(), with(|c| c.release_scene = "warm_wit".into()));
        h.sync.activate("a");
        h.clear();
        h.sync.release();
        assert_eq!(h.relay().post_urls(), [scene_endpoint(BASE, "warm_wit")]);
    }

    /// The port's version of the main process's "unparseable response":
    /// a GET that answers something that is not a /status body leaves no
    /// snapshot, so release falls through rather than restoring nothing.
    #[test]
    fn a_status_body_it_cannot_use_leaves_no_snapshot() {
        let mut h = H::new(Relay::with_status(json!({ "ok": true })), with(|c| c.release_scene = "warm_wit".into()));
        h.sync.activate("a");
        assert!(!h.sync.status().has_prior_state);
        h.clear();
        h.sync.release();
        assert_eq!(h.relay().post_urls(), [scene_endpoint(BASE, "warm_wit")]);
    }

    #[test]
    fn sends_nothing_when_the_read_failed_and_no_release_scene_is_set() {
        let mut h = H::new(read_fails(), config());
        h.sync.activate("a");
        h.clear();
        h.sync.release();
        assert!(h.relay().posts().is_empty());
    }

    #[test]
    fn clears_the_snapshot_so_a_second_release_does_not_re_post_it() {
        let mut h = H::new(Relay::with_status(default_status()), config());
        h.sync.activate("a");
        h.sync.release();
        h.clear();
        h.sync.release();
        assert!(h.relay().posts().is_empty());
        assert!(!h.sync.status().has_prior_state);
    }

    #[test]
    fn takes_a_fresh_snapshot_on_the_next_activation() {
        let mut h = H::new(Relay::with_status(uniform_status([1, 2, 3], 1.0, Value::Null)), config());
        h.sync.activate("a");
        h.sync.release();
        h.sync.activate("a");
        assert_eq!(h.relay().gets(), 2);
    }
}

// =========================================== artnet-spatial.test.js (client)

fn layout_strips() -> Value {
    json!([
        { "name": "u0_01", "pixels": 8, "angle": 180, "x": 6.0, "y": 4.5, "x1": 6.5, "y1": 4.5, "x2": 5.5, "y2": 4.5 },
        { "name": "u0_02", "pixels": 8, "angle": 90, "x": 6.5, "y": 5.0, "x1": 6.5, "y1": 4.5, "x2": 6.5, "y2": 5.5 },
        { "name": "u0_05", "pixels": 8, "angle": 0, "x": 5.0, "y": 6.5, "x1": 4.5, "y1": 6.5, "x2": 5.5, "y2": 6.5 },
        { "name": "u1_01", "pixels": 8, "angle": 0, "x": 1.5, "y": 3.0, "x1": 1.5, "y1": 3.0, "x2": 2.5, "y2": 3.0 },
        { "name": "u4_08", "pixels": 8, "angle": 180, "x": 10.5, "y": 5.5, "x1": 10.5, "y1": 5.5, "x2": 9.5, "y2": 5.5 }
    ])
}

/// A relay with a layout, a one-strip /status, and optionally failing parts.
fn spatial_relay(layout: Option<Value>, pixels_fail: bool) -> Relay {
    Relay {
        calls: Vec::new(),
        post: Box::new(move |url| if pixels_fail && url.ends_with("/pixels") { Err("HTTP 500".into()) } else { Ok(()) }),
        get: Box::new(move |url| {
            if url.ends_with("/layout") {
                return layout.clone().ok_or_else(|| "Failed to fetch".to_owned());
            }
            Ok(json!({ "strips": [{ "name": "a", "rgb": [0, 0, 0], "brightness": 1 }] }))
        }),
    }
}

fn spatial_config() -> Config {
    with(|c| { c.max_brightness = 0.8; c.scene_by_saver = by_saver(json!({ "a": "spatial" })); })
}

fn split_frame() -> Vec<u8> {
    grid(|tx, _| if tx < GRID.tiles_x / 2 { [0, 200, 0] } else { [200, 0, 0] })
}

fn layout_reads(h: &mut H) -> usize {
    let url = layout_endpoint(BASE);
    h.relay().calls.iter().filter(|c| c.url == url).count()
}

mod the_client_in_spatial_mode {
    use super::*;

    fn good() -> H {
        H::new(spatial_relay(Some(json!({ "strips": layout_strips() })), false), spatial_config())
    }

    #[test]
    fn reads_the_layout_once_then_writes_pixels() {
        let mut h = good();
        h.sync.activate("a");
        h.clear();
        h.offer(&split_frame());
        assert_eq!(layout_reads(&mut h), 1);
        assert_eq!(h.relay().post_urls()[0], strip_pixels_endpoint(BASE, "u0_01"));
    }

    #[test]
    fn does_not_re_read_the_layout_on_later_frames() {
        let mut h = good();
        h.sync.activate("a");
        for _ in 0..3 {
            h.send(&split_frame());
        }
        assert_eq!(layout_reads(&mut h), 1);
    }

    #[test]
    fn sends_the_pixel_array_brightness_and_a_transition() {
        let mut h = good();
        h.sync.activate("a");
        h.clear();
        h.offer(&split_frame());
        let body = h.body(0);
        assert_eq!(body["pixels"].as_array().unwrap().len(), 8);
        assert_eq!(body["pixels"][0].as_array().unwrap().len(), 3);
        assert_eq!(f(&body, "brightness"), 0.8);
        assert!(f(&body, "transition_ms") > 0.0);
    }

    #[test]
    fn stops_writing_once_the_picture_settles() {
        let mut h = good();
        h.sync.activate("a");
        h.send(&split_frame());
        h.clear();
        h.offer(&split_frame());
        assert!(h.relay().posts().is_empty());
    }

    #[test]
    fn gives_up_on_a_layout_it_cannot_read_rather_than_asking_every_second() {
        let mut h = H::new(spatial_relay(None, false), spatial_config());
        h.sync.activate("a");
        h.clear();
        for _ in 0..4 {
            h.send(&split_frame());
        }
        assert_eq!(layout_reads(&mut h), 1);
        assert!(h.relay().posts().is_empty());
    }

    #[test]
    fn gives_up_on_a_layout_with_no_usable_geometry() {
        let mut h = H::new(spatial_relay(Some(json!({ "strips": [{ "name": "a" }] })), false), spatial_config());
        h.sync.activate("a");
        h.clear();
        h.offer(&split_frame());
        assert!(h.relay().posts().is_empty());
    }

    #[test]
    fn forgets_what_it_wrote_on_release() {
        let mut h = good();
        h.sync.activate("a");
        h.offer(&split_frame());
        h.sync.release();
        h.clear();
        h.sync.activate("a");
        // Past the 1 Hz gate, which is shared across activations.
        h.now += SEND_INTERVAL_MS;
        h.offer(&split_frame());
        assert!(!h.relay().posts().is_empty());
    }

    #[test]
    fn retries_a_strip_whose_write_failed_instead_of_assuming_it_landed() {
        let mut h = H::new(spatial_relay(Some(json!({ "strips": layout_strips() })), true), spatial_config());
        h.sync.activate("a");
        h.clear();
        h.offer(&split_frame());
        let first = h.relay().post_urls();
        assert_eq!(first.len(), 1, "stops at the first failure");
        // Past the failure backoff, not just the send interval.
        h.now += 10_000;
        h.clear();
        h.offer(&split_frame());
        assert_eq!(h.relay().post_urls(), first);
    }

    #[test]
    fn is_selectable_as_a_global_target_as_well_as_per_saver() {
        let mut h = H::new(spatial_relay(Some(json!({ "strips": layout_strips() })), false),
            with(|c| { c.target = "spatial".into(); c.max_brightness = 0.8; }));
        h.sync.activate("z");
        h.clear();
        h.offer(&split_frame());
        assert!(h.relay().post_urls()[0].contains("/pixels"));
    }
}

// ===================================== artnet-effect-params.test.js (client)

mod undriven_effects_are_started_and_left_alone {
    use super::*;

    #[test]
    fn starts_a_non_field_effect_once_and_then_stops_talking_to_it() {
        let mut h = H::new(Relay::ok(), mapped(json!({ "a": "effect:fire" })));
        h.sync.activate("a");
        h.clear();
        for _ in 0..4 {
            h.send(&frame());
        }
        assert_eq!(h.relay().post_urls(), [format!("{BASE}/effects/fire")]);
    }

    #[test]
    fn nudges_a_field_effect_on_every_send_after_the_first() {
        let mut h = H::new(Relay::ok(), mapped(json!({ "a": "effect:plasma" })));
        h.sync.activate("a");
        h.clear();
        for _ in 0..3 {
            h.send(&frame());
        }
        assert_eq!(h.relay().post_urls(), [
            format!("{BASE}/effects/plasma"), format!("{BASE}/field/params"), format!("{BASE}/field/params"),
        ]);
    }

    #[test]
    fn sends_plasmas_real_parameters_over_the_wire_not_the_spots() {
        let mut h = H::new(Relay::ok(), mapped(json!({ "a": "effect:plasma" })));
        h.sync.activate("a");
        h.clear();
        h.offer(&frame());
        assert_eq!(keys(&h.body(0)), ["brightness", "scale", "speed"]);
    }
}

// ============================================================== catalogue

#[test]
fn reads_scene_names_for_the_dropdowns_and_nothing_when_disabled() {
    let mut h = H::new(Relay::with_status(json!({ "scenes": ["warm_wit", 3, "lab_modus"], "strips": [] })), config());
    let cat = h.sync.read_catalogue().unwrap();
    assert_eq!(cat.scenes, ["warm_wit", "lab_modus"]);
    assert!(cat.effects.iter().any(|e| e == "spot"));
    let mut off = H::new(Relay::ok(), with(|c| c.enabled = false));
    assert_eq!(off.sync.read_catalogue(), None);
    assert!(off.relay().calls.is_empty());
}
