// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Canned readings for reviewing the Weather saver, from `weather-states.js`.
//!
//! Rain, snow, fog and a storm at night cannot be reviewed by waiting for the
//! weather to oblige. Electron cycles these with the preview's A key; here
//! `--weather-state=N|label` pins one. Nothing else reads this file.
//!
//! Plausible rather than extreme values, so the display is checked against
//! readings it will actually receive. Precipitation is a rate, as
//! `weather_source` produces it.

use std::time::Duration;

use crate::weather_source::Reading;

pub struct CannedState {
    pub label: &'static str,
    pub reading: Reading,
}

#[allow(clippy::too_many_arguments)]
const fn canned(label: &'static str, temperature_c: f64, precipitation_mm_h: f64, wind_speed_kmh: f64,
    wind_direction_deg: f64, cloud_cover_pct: f64, weather_code: f64, is_day: bool) -> CannedState {
    CannedState {
        label,
        reading: Reading {
            observed_at: None,
            temperature_c,
            precipitation_mm_h,
            wind_speed_kmh,
            wind_direction_deg,
            cloud_cover_pct,
            weather_code,
            is_day,
            age: Duration::ZERO,
            stale: false,
            synthetic: false,
        },
    }
}

const fn stale(mut s: CannedState, age: Duration) -> CannedState {
    s.reading.age = age;
    s.reading.stale = true;
    s
}

pub const STATES: &[CannedState] = &[
    canned("clear day", 24.3, 0.0, 7.0, 200.0, 4.0, 0.0, true),
    canned("partly cloudy, breezy", 17.1, 0.0, 26.0, 250.0, 55.0, 2.0, true),
    canned("overcast drizzle", 11.4, 0.4, 14.0, 230.0, 96.0, 53.0, true),
    canned("heavy rain, strong wind", 9.8, 7.5, 48.0, 265.0, 100.0, 65.0, true),
    canned("thunderstorm at night", 16.2, 5.0, 33.0, 210.0, 98.0, 95.0, false),
    canned("snow, calm", -2.5, 1.4, 5.0, 20.0, 90.0, 73.0, true),
    canned("fog", 4.0, 0.0, 2.0, 100.0, 100.0, 45.0, true),
    canned("clear night", 8.9, 0.0, 9.0, 180.0, 8.0, 0.0, false),
    // The case a viewer must tell from calm weather: still animating, but
    // the numbers are hours old.
    stale(canned("stale reading", 13.0, 0.0, 11.0, 190.0, 40.0, 1.0, true), Duration::from_secs(3 * 60 * 60)),
];

fn key(s: &str) -> String {
    s.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect()
}

/// A state by index, by label, or by a label prefix only one state has --
/// ignoring case, spaces and punctuation, so `snow`, `clear-night` and
/// `"heavy rain, strong wind"` all work.
pub fn find(selector: &str) -> Option<usize> {
    if let Ok(i) = selector.trim().parse::<usize>() {
        return (i < STATES.len()).then_some(i);
    }
    let want = key(selector);
    if want.is_empty() {
        return None;
    }
    if let Some(i) = STATES.iter().position(|s| key(s.label) == want) {
        return Some(i);
    }
    let mut prefixed = STATES.iter().enumerate().filter(|(_, s)| key(s.label).starts_with(&want));
    match (prefixed.next(), prefixed.next()) {
        (Some((i, _)), None) => Some(i),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_by_index_label_or_unique_prefix() {
        assert_eq!(find("0"), Some(0));
        assert_eq!(find("8"), Some(8));
        assert_eq!(find("9"), None);
        assert_eq!(find("fog"), Some(6));
        assert_eq!(find("Clear-Night"), Some(7));
        assert_eq!(find("heavy rain, strong wind"), Some(3));
        assert_eq!(find("snow"), Some(5));
        assert_eq!(find("thunder"), Some(4));
        // Two states start with "clear".
        assert_eq!(find("clear"), None);
        assert_eq!(find("tornado"), None);
        assert_eq!(find(""), None);
    }

    #[test]
    fn only_the_stale_state_is_stale() {
        let stale: Vec<_> = STATES.iter().filter(|s| s.reading.stale).map(|s| s.label).collect();
        assert_eq!(stale, ["stale reading"]);
    }
}
