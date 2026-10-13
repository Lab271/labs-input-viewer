// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Where a request goes and what a saver asks of the room: the relay's
//! endpoints, `artnetTarget` parsing, the per-saver modes and the `/status`
//! snapshot. Ported from artnet-sync.js with its tests; pure.

use serde_json::{Map, Value};

/// The relay's paths, all relative to the configured base URL.
fn base(url: &str) -> &str {
    url.trim_end_matches('/')
}

/// JS `encodeURIComponent`: a name is encoded, never spliced into the path.
pub fn encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The effect in an `effect:<name>` target, or None for a colour target.
pub fn effect_name_for(target: &str) -> Option<&str> {
    let name = target.trim().strip_prefix("effect:")?.trim();
    (!name.is_empty()).then_some(name)
}

/// `/all`, `/groups/{name}` or `/strips/{name}`, all taking the same
/// ColorRequest. `all` is the default because it is the only target that
/// exists on every install. An unrecognised target falls back to `/all`
/// rather than a made-up path that would 404 every second and look like the
/// relay was broken.
pub fn colour_endpoint(url: &str, target: &str) -> String {
    let base = base(url);
    let t = target.trim();
    if let Some((kind, rest)) = t.split_once(':') {
        let name = encode_component(rest);
        match kind {
            "group" if !name.is_empty() => return format!("{base}/groups/{name}"),
            "strip" if !name.is_empty() => return format!("{base}/strips/{name}"),
            _ => {}
        }
    }
    format!("{base}/all")
}

pub fn layout_endpoint(url: &str) -> String {
    format!("{}/layout", base(url))
}
pub fn strip_pixels_endpoint(url: &str, name: &str) -> String {
    format!("{}/strips/{}/pixels", base(url), encode_component(name))
}
pub fn status_endpoint(url: &str) -> String {
    format!("{}/status", base(url))
}
pub fn scene_endpoint(url: &str, name: &str) -> String {
    format!("{}/scenes/{}", base(url), encode_component(name))
}
pub fn strip_endpoint(url: &str, name: &str) -> String {
    format!("{}/strips/{}", base(url), encode_component(name))
}
/// Starts an effect; body is a flat EffectRequest.
pub fn effect_endpoint(url: &str, name: &str) -> String {
    format!("{}/effects/{}", base(url), encode_component(name))
}
/// Partial update of the running field effect.
pub fn field_params_endpoint(url: &str) -> String {
    format!("{}/field/params", base(url))
}
/// Ends whatever effect is running, leaving the last frame lit.
pub fn stop_endpoint(url: &str) -> String {
    format!("{}/stop", base(url))
}

/// Savers the relay has a genuinely matching effect for. Defaults, not
/// overrides: an explicit entry in `artnetSceneBySaver` always wins,
/// including an explicit `reactive`. Keyed by display name, as Electron's.
pub const DEFAULT_SAVER_MODES: &[(&str, &str)] = &[
    ("Plasma", "effect:plasma"),
    ("Metaballs", "effect:blobs"),
    ("Wave Tank", "effect:ripple"),
];

pub fn default_saver_mode(saver: &str) -> Option<&'static str> {
    DEFAULT_SAVER_MODES.iter().find(|(s, _)| *s == saver).map(|(_, m)| *m)
}

/// What the room does for one saver.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Mode {
    /// Driven from the picture, through the global `artnetTarget`.
    #[default]
    Reactive,
    /// Leave the room alone.
    Off,
    /// The wall's colours on the fixtures nearest them.
    Spatial,
    /// Posted once when the saver starts.
    Scene(String),
    /// This effect, overriding the global target.
    Effect(String),
}

impl Mode {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Reactive => "reactive",
            Self::Off => "off",
            Self::Spatial => "spatial",
            Self::Scene(_) => "scene",
            Self::Effect(_) => "effect",
        }
    }
}

/// artnet-sync.js `modeForSaver`. A present entry wins even when it is
/// `reactive` or empty -- `hasOwnProperty`, not truthiness, or opting out of
/// a built-in pairing would be impossible. Anything unrecognised is reactive
/// rather than silence: a typo in a hand-edited map should look like the old
/// behaviour, not like the lighting having quietly stopped.
pub fn mode_for_saver(saver: &str, by_saver: &Map<String, Value>) -> Mode {
    let configured = match by_saver.get(saver) {
        Some(v) => v.as_str(),
        None => default_saver_mode(saver),
    };
    let raw = configured.filter(|s| !s.is_empty()).unwrap_or("reactive").trim();
    match raw {
        "off" => return Mode::Off,
        "spatial" => return Mode::Spatial,
        _ => {}
    }
    if let Some(name) = raw.strip_prefix("scene:").map(str::trim).filter(|n| !n.is_empty()) {
        return Mode::Scene(name.to_owned());
    }
    if let Some(name) = raw.strip_prefix("effect:").map(str::trim).filter(|n| !n.is_empty()) {
        return Mode::Effect(name.to_owned());
    }
    Mode::Reactive
}

/// What the per-saver dropdown shows: the effective mode, so a paired saver
/// with no entry reads as its effect, not as "Reactive" -- that would state
/// the opposite of what the room is doing. renderer.js uses `??` here, so a
/// null entry shows the pairing.
pub fn saver_select_value(saver: &str, by_saver: &Map<String, Value>) -> String {
    match by_saver.get(saver) {
        Some(Value::Null) | None => default_saver_mode(saver).unwrap_or("reactive").to_owned(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// renderer.js `setArtnetSaverMode`. Reactive is stored as an absent key so
/// the map only holds real decisions -- except for a paired saver, where
/// deleting the key would fall back to the pairing and picking Reactive
/// would appear to do nothing.
pub fn set_saver_mode(by_saver: &mut Map<String, Value>, saver: &str, mode: &str) {
    if mode == "reactive" && default_saver_mode(saver).is_none() {
        by_saver.remove(saver);
    } else {
        by_saver.insert(saver.to_owned(), Value::String(mode.to_owned()));
    }
}

/// One strip in a `/status` snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct StripState {
    pub name: String,
    pub rgb: [i64; 3],
    pub brightness: f64,
}

/// The least needed to put the room back.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub strips: Vec<StripState>,
    /// An effect that was already running, restarted by name on release.
    pub effect: Option<String>,
    /// Every strip the same: one `/all` restores the room instead of forty
    /// POSTs. The common case by far -- a room on one scene, or off.
    pub uniform: Option<([i64; 3], f64)>,
}

/// JS `Number(x) || 0`.
fn number(v: &Value) -> i64 {
    let n = match v {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::String(s) => s.trim().parse().unwrap_or(0.0),
        Value::Bool(b) => f64::from(u8::from(*b)),
        _ => 0.0,
    };
    if n.is_finite() { n.round() as i64 } else { 0 }
}

/// artnet-sync.js `summariseState`: reduce `GET /status` to a snapshot, or
/// None for a body that cannot be used. Lights-off is a real state, not
/// nothing: the wall's room sits at (0,0,0).
pub fn summarise_state(status: &Value) -> Option<Snapshot> {
    let strips: Vec<StripState> = status.get("strips")?.as_array()?
        .iter()
        .filter_map(|st| {
            let name = st.get("name")?.as_str()?;
            let rgb = st.get("rgb")?.as_array()?;
            let at = |i: usize| rgb.get(i).map_or(0, number);
            Some(StripState {
                name: name.to_owned(),
                rgb: [at(0), at(1), at(2)],
                brightness: st.get("brightness").and_then(Value::as_f64).unwrap_or(1.0),
            })
        })
        .collect();
    let first = strips.first()?.clone();
    let effect = match status.get("effect") {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::Object(o)) => o.get("name").and_then(Value::as_str).map(str::to_owned),
        _ => None,
    };
    let uniform = strips.iter()
        .all(|s| s.rgb == first.rgb && s.brightness == first.brightness)
        .then_some((first.rgb, first.brightness));
    Some(Snapshot { strips, effect, uniform })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn map(v: Value) -> Map<String, Value> {
        v.as_object().cloned().unwrap()
    }

    // ---- artnet-sync.test.js: colourEndpoint

    #[test]
    fn builds_the_paths_the_relay_actually_exposes() {
        let base = "http://pi.labs:8000";
        assert_eq!(colour_endpoint(base, "all"), format!("{base}/all"));
        assert_eq!(colour_endpoint(base, "group:universe_0"), format!("{base}/groups/universe_0"));
        assert_eq!(colour_endpoint(base, "strip:bar_left"), format!("{base}/strips/bar_left"));
    }

    #[test]
    fn tolerates_a_trailing_slash_on_the_base_url() {
        assert_eq!(colour_endpoint("http://pi.labs:8000/", "all"), "http://pi.labs:8000/all");
    }

    #[test]
    fn falls_back_to_all_for_an_unrecognised_target() {
        assert_eq!(colour_endpoint("http://x", "nonsense"), "http://x/all");
        assert_eq!(colour_endpoint("http://x", "group:"), "http://x/all");
        assert_eq!(colour_endpoint("http://x", ""), "http://x/all");
    }

    #[test]
    fn encodes_a_name_with_awkward_characters() {
        assert_eq!(colour_endpoint("http://x", "group:back wall"), "http://x/groups/back%20wall");
    }

    // ---- artnet-spot.test.js: effectNameFor, endpoints

    #[test]
    fn recognises_effect_targets_only() {
        assert_eq!(effect_name_for("effect:spot"), Some("spot"));
        assert_eq!(effect_name_for("  effect:aurora  "), Some("aurora"));
        for t in ["all", "group:universe_0", "strip:u0_01", "effect:", ""] {
            assert_eq!(effect_name_for(t), None, "{t}");
        }
    }

    #[test]
    fn builds_effect_field_and_stop_paths_and_tolerates_a_trailing_slash() {
        assert_eq!(effect_endpoint("http://pi:8000/", "spot"), "http://pi:8000/effects/spot");
        assert_eq!(field_params_endpoint("http://pi:8000//"), "http://pi:8000/field/params");
        assert_eq!(stop_endpoint("http://pi:8000"), "http://pi:8000/stop");
    }

    #[test]
    fn encodes_a_name_rather_than_splicing_it_into_the_path() {
        assert_eq!(effect_endpoint("http://pi:8000", "a/b"), "http://pi:8000/effects/a%2Fb");
    }

    // ---- artnet-per-saver.test.js: modeForSaver

    #[test]
    fn defaults_to_reactive_so_an_empty_map_changes_nothing() {
        assert_eq!(mode_for_saver("plasma", &Map::new()), Mode::Reactive);
        assert_eq!(mode_for_saver("plasma", &map(json!({ "plasma": "reactive" }))), Mode::Reactive);
    }

    #[test]
    fn reads_scene_effect_and_off() {
        assert_eq!(mode_for_saver("a", &map(json!({ "a": "scene:warm_wit" }))), Mode::Scene("warm_wit".into()));
        assert_eq!(mode_for_saver("a", &map(json!({ "a": "effect:spot" }))), Mode::Effect("spot".into()));
        assert_eq!(mode_for_saver("a", &map(json!({ "a": "off" }))), Mode::Off);
    }

    #[test]
    fn falls_back_to_reactive_on_a_malformed_entry_not_to_silence() {
        for bad in [json!("scene:"), json!("effect:"), json!("nonsense"), json!(""), json!("scene"), Value::Null] {
            assert_eq!(mode_for_saver("a", &map(json!({ "a": bad }))), Mode::Reactive);
        }
    }

    #[test]
    fn only_applies_the_entry_for_the_saver_asked_about() {
        let m = map(json!({ "plasma": "off" }));
        assert_eq!(mode_for_saver("plasma", &m), Mode::Off);
        assert_eq!(mode_for_saver("frost", &m), Mode::Reactive);
    }

    // ---- artnet-spatial.test.js

    #[test]
    fn spatial_is_recognised_by_mode_for_saver() {
        assert_eq!(mode_for_saver("a", &map(json!({ "a": "spatial" }))), Mode::Spatial);
    }

    // ---- artnet-effect-params.test.js: default saver pairings

    #[test]
    fn pairs_plasma_metaballs_and_wave_tank() {
        assert_eq!(mode_for_saver("Plasma", &Map::new()), Mode::Effect("plasma".into()));
        assert_eq!(mode_for_saver("Metaballs", &Map::new()), Mode::Effect("blobs".into()));
        assert_eq!(mode_for_saver("Wave Tank", &Map::new()), Mode::Effect("ripple".into()));
    }

    #[test]
    fn leaves_every_other_saver_reactive() {
        for s in ["Matrix Rain", "Frost", "Boids", "Voronoi", "DVD Logo"] {
            assert_eq!(mode_for_saver(s, &Map::new()), Mode::Reactive, "{s}");
        }
    }

    #[test]
    fn a_present_but_empty_entry_is_reactive_not_absent() {
        for empty in [json!(""), Value::Null] {
            assert_eq!(mode_for_saver("Plasma", &map(json!({ "Plasma": empty }))), Mode::Reactive);
        }
    }

    #[test]
    fn an_explicit_entry_beats_the_built_in_pairing() {
        assert_eq!(mode_for_saver("Plasma", &map(json!({ "Plasma": "reactive" }))), Mode::Reactive);
        assert_eq!(mode_for_saver("Plasma", &map(json!({ "Plasma": "off" }))), Mode::Off);
        assert_eq!(mode_for_saver("Plasma", &map(json!({ "Plasma": "scene:warm_wit" }))), Mode::Scene("warm_wit".into()));
    }

    #[test]
    fn only_pairs_savers_that_have_a_driven_effect() {
        for (_, mode) in DEFAULT_SAVER_MODES {
            let name = mode.strip_prefix("effect:").expect(mode);
            assert!(super::super::effects::is_driven(name), "{name}");
        }
    }

    // ---- artnet-settings-ui.test.js: the per-saver list

    #[test]
    fn the_dropdown_shows_the_pairing_a_saver_is_actually_running() {
        assert_eq!(saver_select_value("Plasma", &Map::new()), "effect:plasma");
        assert_eq!(saver_select_value("Frost", &Map::new()), "reactive");
    }

    #[test]
    fn writes_reactive_for_a_paired_saver_so_choosing_it_takes_effect() {
        let mut m = Map::new();
        set_saver_mode(&mut m, "Plasma", "reactive");
        assert_eq!(m["Plasma"], "reactive");
        assert_eq!(saver_select_value("Plasma", &m), "reactive");
    }

    #[test]
    fn stores_an_unpaired_savers_reactive_as_an_absent_key() {
        let mut m = map(json!({ "Frost": "off" }));
        set_saver_mode(&mut m, "Frost", "reactive");
        assert!(!m.contains_key("Frost"));
    }

    // ---- artnet-per-saver.test.js: summariseState

    fn uniform_status(rgb: [i64; 3], brightness: f64, effect: Value) -> Value {
        let strips: Vec<Value> = (1..=40).map(|i| json!({ "name": format!("u0_{i:02}"), "rgb": rgb, "brightness": brightness })).collect();
        json!({ "effect": effect, "strips": strips })
    }

    fn mixed_status() -> Value {
        let mut s = uniform_status([40, 10, 80], 0.6, Value::Null);
        s["strips"][7] = json!({ "name": "u0_08", "rgb": [255, 0, 0], "brightness": 1 });
        s
    }

    #[test]
    fn detects_a_uniform_room_so_one_all_can_restore_it() {
        let sum = summarise_state(&uniform_status([40, 10, 80], 0.6, Value::Null)).unwrap();
        assert_eq!(sum.uniform, Some(([40, 10, 80], 0.6)));
        assert_eq!(sum.strips.len(), 40);
    }

    #[test]
    fn refuses_to_call_a_mixed_room_uniform() {
        assert_eq!(summarise_state(&mixed_status()).unwrap().uniform, None);
    }

    #[test]
    fn treats_a_brightness_only_difference_as_mixed() {
        let mut st = uniform_status([40, 10, 80], 0.6, Value::Null);
        st["strips"][3] = json!({ "name": "u0_04", "rgb": [40, 10, 80], "brightness": 0.2 });
        let sum = summarise_state(&st).unwrap();
        assert_eq!(sum.uniform, None);
        assert_eq!(sum.strips[3].brightness, 0.2);
    }

    #[test]
    fn keeps_a_running_effect_by_name_from_either_shape() {
        assert_eq!(summarise_state(&uniform_status([0, 0, 0], 1.0, json!("aurora"))).unwrap().effect.as_deref(), Some("aurora"));
        assert_eq!(summarise_state(&uniform_status([0, 0, 0], 1.0, json!({ "name": "ripple" }))).unwrap().effect.as_deref(), Some("ripple"));
        assert_eq!(summarise_state(&uniform_status([0, 0, 0], 1.0, Value::Null)).unwrap().effect, None);
    }

    #[test]
    fn returns_none_for_a_body_it_cannot_use() {
        for body in [Value::Null, json!({}), json!({ "strips": [] }), json!({ "strips": "nope" })] {
            assert_eq!(summarise_state(&body), None, "{body}");
        }
    }

    #[test]
    fn treats_lights_off_as_a_real_state_worth_restoring() {
        let sum = summarise_state(&uniform_status([0, 0, 0], 1.0, Value::Null)).unwrap();
        assert_eq!(sum.uniform, Some(([0, 0, 0], 1.0)));
    }
}
