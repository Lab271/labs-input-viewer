// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Test-mode launch flags, the Rust side of #248.
//!
//! Same three flags as the Electron app, same semantics:
//!
//! | Flag | Effect |
//! |---|---|
//! | `--mock[=N]` | N synthetic inputs (1-8, default 4). No capture hardware. Each also has a synthetic audio device, `Digital Audio Interface (Mock Input N)`, playing a sine tone of its own pitch (220, 277, 330, 440 Hz...), so pairing, mixing and per-half volume can be heard and read in the log (`[Audio] output rms ...` every 5 s) without hardware. |
//! | `--no-signal` | Pins every input into the no-signal state. |
//! | `--screensaver-delay=MS` | Replaces the five-minute wait. |
//! | `--screensaver=NAME` | Prototype only: pins the first saver picked (`plasma`, `juliafamily`, `starfield`, ...). |
//! | `--test-pattern[=WxH@FPS]` | Prototype only: mock inputs carry a motion test pattern, 3840x2160@60 by default. Implies `--mock=2`, the wall's two cards. |
//! | `--hud` | Prototype only: per-half frame-rate and latency overlay, and a `[render]` log line every 5s. |
//! | `--mock-shape=W:H` | Prototype only: mock inputs letterboxed at that shape inside pure black, as the cards fit a source of another shape -- to watch cropping (ADR 0015). |
//! | `--weather-state=N\|LABEL` | Prototype only: Weather shows a canned reading from `weather_states.rs` -- by index, label, or a label prefix only one state has, such as `snow` -- instead of the live one, so rain, snow, fog and night can be reviewed. Electron does this with the preview's A key. |
//! | `--mock-fault=dark\|stall\|fail` | Prototype only: the last mock input goes dark, stalls after 10 s, or fails to open -- to watch the health policy reopen it. |
//!
//! Unknown `--` arguments are collected rather than ignored, so a typo like
//! `--nosignal` gets logged. A flag that silently does nothing means someone at
//! the wall is looking at a production screen believing it is in test mode.

use std::time::Duration;

use crate::capture::{MockFault, PatternFormat};

pub const DEFAULT_MOCK_COUNT: usize = 4;
pub const MAX_MOCK_COUNT: usize = 8;
pub const DEFAULT_SCREENSAVER_DELAY: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, PartialEq)]
pub struct Flags {
    /// `Some(n)` when `--mock` was given.
    pub mock: Option<usize>,
    pub no_signal: bool,
    pub screensaver_delay: Duration,
    /// Normalised as the preview selector does: lowercase, no spaces, `-` or `_`.
    pub screensaver: Option<String>,
    pub test_pattern: Option<PatternFormat>,
    pub hud: bool,
    pub mock_fault: Option<MockFault>,
    pub mock_shape: Option<f64>,
    /// Index into `weather_states::STATES`.
    pub weather_state: Option<usize>,
    pub unrecognised: Vec<String>,
}

impl Default for Flags {
    fn default() -> Self {
        Self {
            mock: None,
            no_signal: false,
            screensaver_delay: DEFAULT_SCREENSAVER_DELAY,
            screensaver: None,
            test_pattern: None,
            hud: false,
            mock_fault: None,
            mock_shape: None,
            weather_state: None,
            unrecognised: Vec::new(),
        }
    }
}

pub fn parse<I, S>(args: I) -> Flags
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut flags = Flags::default();
    for arg in args {
        let arg = arg.as_ref();
        if !arg.starts_with("--") {
            continue;
        }
        let (name, value) = match arg.split_once('=') {
            Some((n, v)) => (n, Some(v)),
            None => (arg, None),
        };
        match (name, value) {
            ("--mock", None) => flags.mock = Some(DEFAULT_MOCK_COUNT),
            ("--mock", Some(v)) => match v.parse::<usize>() {
                Ok(n) => flags.mock = Some(n.clamp(1, MAX_MOCK_COUNT)),
                Err(_) => flags.unrecognised.push(arg.to_owned()),
            },
            ("--no-signal", None) => flags.no_signal = true,
            ("--screensaver-delay", Some(v)) => match v.parse::<u64>() {
                Ok(ms) => flags.screensaver_delay = Duration::from_millis(ms),
                Err(_) => flags.unrecognised.push(arg.to_owned()),
            },
            ("--screensaver", Some(v)) if !v.is_empty() => flags.screensaver = Some(normalise(v)),
            ("--test-pattern", None) => flags.test_pattern = Some(DEFAULT_PATTERN),
            ("--test-pattern", Some(v)) => match parse_format(v) {
                Some(f) => flags.test_pattern = Some(f),
                None => flags.unrecognised.push(arg.to_owned()),
            },
            ("--hud", None) => flags.hud = true,
            ("--mock-shape", Some(v)) => match v.split_once(':').and_then(|(a, b)| Some((a.parse::<f64>().ok()?, b.parse::<f64>().ok()?))) {
                Some((a, b)) if a > 0.0 && b > 0.0 => flags.mock_shape = Some(a / b),
                _ => flags.unrecognised.push(arg.to_owned()),
            },
            ("--weather-state", Some(v)) => match crate::weather_states::find(v) {
                Some(i) => flags.weather_state = Some(i),
                None => flags.unrecognised.push(arg.to_owned()),
            },
            ("--mock-fault", Some(v)) => match v {
                "dark" => flags.mock_fault = Some(MockFault::Dark),
                "stall" => flags.mock_fault = Some(MockFault::Stall),
                "fail" => flags.mock_fault = Some(MockFault::Fail),
                _ => flags.unrecognised.push(arg.to_owned()),
            },
            _ => flags.unrecognised.push(arg.to_owned()),
        }
    }
    // A test pattern needs inputs to carry it: two, like the wall. So does a
    // fault, with a healthy input beside the broken one.
    if (flags.test_pattern.is_some() || flags.mock_fault.is_some() || flags.mock_shape.is_some()) && flags.mock.is_none() {
        flags.mock = Some(2);
    }
    flags
}

pub const DEFAULT_PATTERN: PatternFormat = PatternFormat { width: 3840, height: 2160, fps: 60 };

/// `WxH@FPS`, `WxH` (60 fps) or the shorthands `4k60`, `4k30`, `1080p60`.
pub fn parse_format(v: &str) -> Option<PatternFormat> {
    let named = match v.to_ascii_lowercase().as_str() {
        "4k60" => Some((3840, 2160, 60)),
        "4k30" => Some((3840, 2160, 30)),
        "1080p60" => Some((1920, 1080, 60)),
        _ => None,
    };
    let (width, height, fps) = match named {
        Some(t) => t,
        None => {
            let (size, fps) = v.split_once('@').unwrap_or((v, "60"));
            let (w, h) = size.split_once('x')?;
            (w.parse().ok()?, h.parse().ok()?, fps.parse().ok()?)
        }
    };
    // 64px is the smallest the pattern lays out legibly; 8K is the largest
    // anything here will meet.
    ((64..=7680).contains(&width) && (64..=4320).contains(&height) && (1..=240).contains(&fps))
        .then_some(PatternFormat { width, height, fps })
}

pub fn normalise(name: &str) -> String {
    name.chars().filter(|c| !matches!(c, ' ' | '-' | '_')).flat_map(char::to_lowercase).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_without_flags() {
        assert_eq!(parse(["input-viewer"]), Flags::default());
    }

    #[test]
    fn mock_count_defaults_and_clamps() {
        assert_eq!(parse(["--mock"]).mock, Some(4));
        assert_eq!(parse(["--mock=2"]).mock, Some(2));
        assert_eq!(parse(["--mock=0"]).mock, Some(1));
        assert_eq!(parse(["--mock=99"]).mock, Some(8));
    }

    #[test]
    fn flags_combine() {
        let f = parse(["--mock=2", "--no-signal", "--screensaver-delay=0"]);
        assert_eq!(f.mock, Some(2));
        assert!(f.no_signal);
        assert_eq!(f.screensaver_delay, Duration::ZERO);
        assert!(f.unrecognised.is_empty());
    }

    #[test]
    fn screensaver_name_is_normalised() {
        assert_eq!(parse(["--screensaver=DVD-logo"]).screensaver.as_deref(), Some("dvdlogo"));
        assert_eq!(parse(["--screensaver="]).unrecognised, ["--screensaver="]);
    }

    #[test]
    fn test_pattern_defaults_to_two_4k60_inputs() {
        let f = parse(["--test-pattern"]);
        assert_eq!(f.test_pattern, Some(DEFAULT_PATTERN));
        assert_eq!(f.mock, Some(2));
        assert_eq!(parse(["--mock=1", "--test-pattern=1080p60"]).mock, Some(1));
    }

    #[test]
    fn pattern_formats() {
        let f = |w, h, fps| Some(PatternFormat { width: w, height: h, fps });
        assert_eq!(parse_format("3840x2160@60"), f(3840, 2160, 60));
        assert_eq!(parse_format("1920x1080"), f(1920, 1080, 60));
        assert_eq!(parse_format("4K30"), f(3840, 2160, 30));
        assert_eq!(parse_format("3840x2160@0"), None);
        assert_eq!(parse_format("huge"), None);
        assert_eq!(parse(["--test-pattern=nope"]).unrecognised, ["--test-pattern=nope"]);
    }

    #[test]
    fn mock_fault_implies_two_mock_inputs() {
        let f = parse(["--mock-fault=stall"]);
        assert_eq!(f.mock_fault, Some(MockFault::Stall));
        assert_eq!(f.mock, Some(2));
        assert_eq!(parse(["--mock-fault=melt"]).unrecognised, ["--mock-fault=melt"]);
    }

    #[test]
    fn mock_shape_is_a_ratio() {
        assert_eq!(parse(["--mock-shape=5:1"]).mock_shape, Some(5.0));
        assert_eq!(parse(["--mock-shape=16:10"]).mock_shape, Some(1.6));
        assert_eq!(parse(["--mock-shape=wide"]).unrecognised, ["--mock-shape=wide"]);
        assert_eq!(parse(["--mock-shape=5:0"]).unrecognised, ["--mock-shape=5:0"]);
    }

    #[test]
    fn weather_state_by_index_or_label() {
        assert_eq!(parse(["--weather-state=3"]).weather_state, Some(3));
        assert_eq!(parse(["--weather-state=clear-night"]).weather_state, Some(7));
        assert_eq!(parse(["--weather-state=snow"]).weather_state, Some(5));
        assert_eq!(parse(["--weather-state=hail"]).unrecognised, ["--weather-state=hail"]);
        assert_eq!(parse(["--weather-state"]).unrecognised, ["--weather-state"]);
    }

    #[test]
    fn typos_are_reported_not_dropped() {
        let f = parse(["--nosignal", "--mock=lots", "--screensaver-delay"]);
        assert_eq!(f.unrecognised, ["--nosignal", "--mock=lots", "--screensaver-delay"]);
        assert!(!f.no_signal);
        assert_eq!(f.mock, None);
    }
}
