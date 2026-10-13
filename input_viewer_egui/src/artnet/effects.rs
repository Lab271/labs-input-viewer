// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! How each relay effect is driven from the picture: artnet-sync.js
//! `EFFECT_DRIVERS`, with artnet-effect-params.test.js.
//!
//! THE PARAMETERS ARE NOT INTERCHANGEABLE. Every effect declares its own set
//! and the relay ignores anything else -- silently, with a 200. Electron
//! 3.1.0 sent the spot's shape to whichever effect was configured, so
//! `effect:plasma` (scale, speed, brightness, no colour) ran at its defaults
//! forever while looking connected.
//!
//! Every driver here is a *field* effect, nudgeable through `/field/params`.
//! An effect with no driver (`fire`, `police`...) is started once and left
//! alone: its parameters can only change by re-POSTing it, which resets its
//! animation phase, so a per-second nudge would be a visible stutter.

use serde_json::{Map, Value, json};

use super::colour::{Focus, Rgb, SPOT_SOFTNESS, round4, spot_diameter};

/// The driven effects, in the order the settings offer them.
pub const DRIVEN: &[&str] = &["spot", "ripple", "plasma", "blobs", "aurora", "sweep", "tunnel"];

#[cfg(test)]
pub fn is_driven(name: &str) -> bool {
    DRIVEN.contains(&name)
}

/// The request body for an effect, or None if it is not one we drive.
///
/// - `col`: the frame's dominant colour, already scaled by maxBrightness
/// - `focus`: the luminance centroid and horizontal spread
/// - `level`: the frame's overall brightness, 0..1
pub fn effect_params(name: &str, col: Rgb, focus: Focus, level: f64, spot_depth: f64) -> Option<Map<String, Value>> {
    let Rgb { r, g, b } = col;
    let spread = focus.spread;
    let v = match name {
        // Positional: a circle of light that follows the bright part of the wall.
        "spot" => json!({
            "r": r, "g": g, "b": b,
            "cx": round4(focus.cx),
            "cy": round4(spot_depth),
            "diameter": round4(spot_diameter(spread)),
            "softness": SPOT_SOFTNESS,
        }),
        // Rings from a moving origin; tracks like the spot does. A
        // concentrated frame gets tight rings, a diffuse one broad ones.
        "ripple" => json!({
            "r": r, "g": g, "b": b,
            "cx": round4(focus.cx),
            "cy": round4(spot_depth),
            "wavelength": round4(0.15 + spread * 0.5),
        }),
        // Generates its own colour field, so only level and character are ours.
        "plasma" => json!({
            "scale": round4(0.6 + spread * 1.4),
            "speed": round4(0.3 + level * 0.7),
            "brightness": round4(level),
        }),
        "aurora" => json!({
            "scale": round4(0.5 + spread * 1.5),
            "speed": round4(0.2 + level * 0.5),
            "brightness": round4(level),
        }),
        // A tight frame reads as few large blobs, a busy one as many small.
        "blobs" => json!({
            "r": r, "g": g, "b": b,
            "size": round4(0.12 + (1.0 - spread) * 0.25),
            "count": (2.0 + spread * 6.0).round().max(2.0) as i64,
            "speed": round4(0.3 + level * 0.6),
        }),
        "sweep" => json!({
            "r": r, "g": g, "b": b,
            "width": round4(0.1 + spread * 0.4),
            "speed": round4(0.3 + level * 0.7),
        }),
        "tunnel" => json!({
            "speed": round4(0.3 + level * 0.7),
            "brightness": round4(level),
            "rings": (3.0 + spread * 5.0).round().max(2.0) as i64,
            "arms": (2.0 + (1.0 - spread) * 4.0).round().max(2.0) as i64,
        }),
        _ => return None,
    };
    v.as_object().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parameters each effect accepts, from the live relay's GET /effects.
    /// Hardcoded on purpose: it is the external contract, and a copy is what
    /// makes drift in our own table detectable.
    const DECLARED: &[(&str, &[&str])] = &[
        ("spot", &["r", "g", "b", "cx", "cy", "diameter", "softness"]),
        ("ripple", &["r", "g", "b", "speed", "wavelength", "cx", "cy"]),
        ("plasma", &["scale", "speed", "brightness"]),
        ("blobs", &["r", "g", "b", "speed", "size", "count"]),
        ("tunnel", &["speed", "rings", "arms", "brightness"]),
        ("sweep", &["r", "g", "b", "speed", "direction", "width"]),
        ("aurora", &["speed", "scale", "brightness"]),
    ];
    /// Effects the relay reports as field-based.
    const FIELD_EFFECTS: &[&str] = &["spot", "ripple", "plasma", "blobs", "tunnel", "sweep", "aurora"];
    const NON_FIELD: &[&str] = &["rainbow", "chase", "breathe", "strobe", "police", "fire", "sparkle", "wave", "comet", "snake"];

    const COL: Rgb = Rgb { r: 200, g: 90, b: 30 };
    const FOCUS: Focus = Focus { cx: 0.4, cy: 0.5, spread: 0.5, weight: 0.5 };

    fn params(name: &str, focus: Focus, level: f64) -> Map<String, Value> {
        effect_params(name, COL, focus, level, 0.5).unwrap_or_else(|| panic!("{name} has no driver"))
    }

    fn f(m: &Map<String, Value>, k: &str) -> f64 {
        m[k].as_f64().unwrap()
    }

    #[test]
    fn every_driven_effect_sends_only_what_it_declares_and_something() {
        for (name, declared) in DECLARED {
            let p = params(name, FOCUS, 0.7);
            let extra: Vec<_> = p.keys().filter(|k| !declared.contains(&k.as_str())).collect();
            assert!(extra.is_empty(), "{name} sent undeclared keys {extra:?}");
            assert!(!p.is_empty(), "{name}");
        }
    }

    #[test]
    fn marks_exactly_the_relays_field_effects_as_driven() {
        for name in DRIVEN {
            assert!(FIELD_EFFECTS.contains(name), "{name} is not a field effect on the relay");
        }
    }

    #[test]
    fn does_not_claim_to_drive_an_effect_it_has_no_entry_for() {
        for name in NON_FIELD {
            assert!(effect_params(name, COL, FOCUS, 0.7, 0.5).is_none(), "{name}");
        }
    }

    #[test]
    fn plasma_sends_scale_speed_brightness_and_no_colour() {
        let p = params("plasma", FOCUS, 0.7);
        let mut keys: Vec<_> = p.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["brightness", "scale", "speed"]);
    }

    #[test]
    fn plasma_tracks_brightness_and_spread() {
        let (dim, bright) = (params("plasma", FOCUS, 0.15), params("plasma", FOCUS, 0.95));
        assert!(f(&bright, "brightness") > f(&dim, "brightness"));
        assert!(f(&bright, "speed") > f(&dim, "speed"));
        let tight = params("plasma", Focus { spread: 0.0, ..FOCUS }, 0.5);
        let wide = params("plasma", Focus { spread: 1.0, ..FOCUS }, 0.5);
        assert!(f(&wide, "scale") > f(&tight, "scale"));
    }

    #[test]
    fn spot_and_ripple_follow_the_luminance_centroid() {
        for name in ["spot", "ripple"] {
            let a = params(name, Focus { cx: 0.1, ..FOCUS }, 0.5);
            let b = params(name, Focus { cx: 0.9, ..FOCUS }, 0.5);
            assert!(f(&b, "cx") > f(&a, "cx"), "{name}");
        }
    }

    #[test]
    fn colour_only_on_the_effects_that_accept_it() {
        for name in ["spot", "ripple", "blobs", "sweep"] {
            let p = params(name, FOCUS, 0.5);
            assert_eq!((&p["r"], &p["g"], &p["b"]), (&json!(200), &json!(90), &json!(30)), "{name}");
        }
        for name in ["plasma", "aurora", "tunnel"] {
            assert!(!params(name, FOCUS, 0.5).contains_key("r"), "{name}");
        }
    }

    #[test]
    fn integer_parameters_stay_integral_at_every_spread() {
        for spread in [0.0, 0.1, 0.3, 0.4, 0.5, 0.7, 0.85, 1.0] {
            let fc = Focus { spread, ..FOCUS };
            assert!(params("blobs", fc, 0.5)["count"].is_i64(), "blobs.count at {spread}");
            let t = params("tunnel", fc, 0.5);
            assert!(t["rings"].is_i64() && t["arms"].is_i64(), "tunnel at {spread}");
        }
    }

    #[test]
    fn fractional_parameters_stay_inside_zero_one() {
        for spread in [0.0, 0.5, 1.0] {
            for level in [0.0, 0.5, 1.0] {
                for name in DRIVEN {
                    let p = params(name, Focus { spread, ..FOCUS }, level);
                    for k in ["cx", "cy", "diameter", "softness", "brightness", "width", "size"] {
                        if let Some(v) = p.get(k).and_then(Value::as_f64) {
                            assert!((0.0..=1.0).contains(&v), "{name}.{k} = {v} at spread {spread} level {level}");
                        }
                    }
                }
            }
        }
    }
}
