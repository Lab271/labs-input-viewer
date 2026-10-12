// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! What the Settings dialog says, kept apart from how it draws it -- the
//! counterpart of `settings-status.js`, and unit tested for the same reason.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Ok,
    Warn,
    Off,
}

/// Where an input stands, as far as the dialog is concerned.
#[derive(Debug, Clone, PartialEq)]
pub enum InputState {
    /// Not on either half, so no stream is open and nothing is known.
    NotOnScreen,
    Opening,
    Live { width: usize, height: usize },
    /// Frames arriving, picture one flat colour: being retried slowly.
    Dark,
    /// Closed for a reopen, about to open again.
    Reopening,
    Failed(String),
}

/// The Key column: `1`-`4` follow the order of *enabled* inputs, so a disabled
/// input has no key (`–`) and the fifth enabled one onward has none either
/// (`·`) -- printing `5` would promise a binding that does not exist.
pub fn input_key_label(enabled_index: Option<usize>) -> &'static str {
    match enabled_index {
        None => "–",
        Some(i) => ["1", "2", "3", "4"].get(i).copied().unwrap_or("·"),
    }
}

pub fn input_status(enabled: bool, state: &InputState) -> (Tone, String) {
    if !enabled {
        return (Tone::Off, "Disabled. Hidden from the pickers and the number keys.".into());
    }
    match state {
        InputState::NotOnScreen => (Tone::Off, "Not on screen.".into()),
        InputState::Opening => (Tone::Off, "Opening…".into()),
        InputState::Live { width, height } => (Tone::Ok, format!("Live, {width}x{height}.")),
        InputState::Dark => (Tone::Warn, "On screen, picture is one flat colour. Retrying.".into()),
        InputState::Reopening => (Tone::Warn, "Reopening…".into()),
        InputState::Failed(e) => (Tone::Warn, format!("{e}. Retrying.")),
    }
}

/// The nav entry for Inputs: how many are enabled, and amber if one on screen
/// is failing or none are enabled at all.
pub fn inputs_nav(states: &[(bool, InputState)]) -> (String, Tone) {
    let enabled = states.iter().filter(|(e, _)| *e).count();
    let text = format!("{enabled} of {}", states.len());
    let failing = states.iter().any(|(e, s)| *e && matches!(s, InputState::Failed(_) | InputState::Dark));
    let tone = if states.is_empty() || enabled == 0 || failing { Tone::Warn } else { Tone::Ok };
    (text, tone)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_labels_follow_enabled_order() {
        let labels: Vec<_> = [None, Some(0), Some(3), Some(4)].into_iter().map(input_key_label).collect();
        assert_eq!(labels, ["–", "1", "4", "·"]);
    }

    #[test]
    fn disabled_wins_over_whatever_the_stream_is_doing() {
        let (tone, line) = input_status(false, &InputState::Failed("busy".into()));
        assert_eq!(tone, Tone::Off);
        assert!(line.starts_with("Disabled"));
    }

    #[test]
    fn live_reports_the_resolution() {
        assert_eq!(input_status(true, &InputState::Live { width: 3840, height: 2160 }),
            (Tone::Ok, "Live, 3840x2160.".into()));
    }

    #[test]
    fn nav_warns_on_failures_and_on_nothing_enabled() {
        assert_eq!(inputs_nav(&[(true, InputState::NotOnScreen), (false, InputState::NotOnScreen)]),
            ("1 of 2".into(), Tone::Ok));
        assert_eq!(inputs_nav(&[(true, InputState::Failed("x".into()))]).1, Tone::Warn);
        assert_eq!(inputs_nav(&[(false, InputState::NotOnScreen)]).1, Tone::Warn);
        // A failing input that is disabled is the operator's choice, not a fault.
        assert_eq!(inputs_nav(&[(true, InputState::NotOnScreen), (false, InputState::Failed("x".into()))]).1, Tone::Ok);
        assert_eq!(inputs_nav(&[]).1, Tone::Warn);
    }
}
