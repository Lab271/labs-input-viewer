// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! What the Settings dialog says, kept apart from how it draws it -- the
//! counterpart of `settings-status.js`, and unit tested for the same reason.

use crate::remote_key::Last;

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

/// The remote keyboard's nav entry, tone and status line.
pub fn remote_keyboard_status(enabled: bool, host: &str, api_key: &str, last: Option<&Last>) -> (&'static str, Tone, String) {
    if !enabled {
        return ("Off", Tone::Off, String::new());
    }
    if host.trim().is_empty() || api_key.is_empty() {
        return ("On", Tone::Warn, "Set a hostname and an API key to send presses.".into());
    }
    let Some(last) = last else { return ("On", Tone::Ok, "Ready. Nothing sent yet.".into()) };
    let when = last.at.format("%H:%M:%S");
    let dir = last.direction.as_str();
    match (last.ok, last.status) {
        (true, Some(s)) => ("On", Tone::Ok, format!("Sent {dir} at {when} (HTTP {s}).")),
        (true, None) => ("On", Tone::Ok, format!("Sent {dir} at {when}.")),
        (false, Some(s)) => ("On", Tone::Warn, format!("Last press failed at {when}: HTTP {s}.")),
        (false, None) => ("On", Tone::Warn,
            format!("Last press failed at {when}: {}.", last.error.as_deref().unwrap_or("no response"))),
    }
}

/// True for an http(s) URL with a host -- settings-status.js `isRelayUrl`,
/// which leaned on `new URL()`. Also what the Art-Net transport accepts, so
/// the status line and the requests cannot disagree.
pub fn is_relay_url(url: &str) -> bool {
    let u = url.trim();
    let rest = ["http://", "https://"].iter().find_map(|s| {
        u.get(..s.len()).filter(|p| p.eq_ignore_ascii_case(s)).map(|_| &u[s.len()..])
    });
    let Some(rest) = rest else { return false };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = if host_port.starts_with('[') {
        host_port.split_inclusive(']').next().unwrap_or("")
    } else {
        host_port.split(':').next().unwrap_or("")
    };
    !host.is_empty() && !host.contains(char::is_whitespace)
}

/// What the Art-Net client last reported, as far as the status line cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtnetCounts<'a> {
    pub sent: u64,
    pub failures: u32,
    pub last_error: Option<&'a str>,
}

/// Art-Net's nav entry, tone and status line -- settings-status.js
/// `artnetStatus`. Before Electron had this, a wrong URL or a dead relay was
/// visible only in the log file.
pub fn artnet_status(enabled: bool, url: &str, counts: Option<ArtnetCounts<'_>>) -> (&'static str, Tone, String) {
    if !enabled {
        return ("Off", Tone::Off, String::new());
    }
    let url = url.trim();
    if url.is_empty() {
        return ("On", Tone::Warn, "Not configured: enter the relay URL. Nothing is sent until you do.".into());
    }
    if !is_relay_url(url) {
        return ("On", Tone::Warn, "That is not an http(s) URL. Nothing is sent.".into());
    }
    match counts {
        Some(ArtnetCounts { failures, last_error: Some(e), .. }) if failures > 0 => {
            ("On", Tone::Warn, format!("Last send failed: {e}."))
        }
        Some(ArtnetCounts { sent, .. }) if sent > 0 => {
            ("On", Tone::Ok, format!("Sending. {sent} update{} so far.", if sent == 1 { "" } else { "s" }))
        }
        _ => ("On", Tone::Ok, "Ready. The lights follow the next screensaver.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote_key::Direction;
    use chrono::TimeZone;

    #[test]
    fn remote_keyboard_off_needs_setup_and_reports_the_last_press() {
        assert_eq!(remote_keyboard_status(false, "kbd", "k", None), ("Off", Tone::Off, String::new()));
        assert_eq!(remote_keyboard_status(true, " ", "k", None).1, Tone::Warn);
        assert_eq!(remote_keyboard_status(true, "kbd", "", None).1, Tone::Warn);
        let at = chrono::Local.with_ymd_and_hms(2026, 10, 13, 14, 2, 3).unwrap();
        let last = |ok, status, error: Option<&str>| Last { ok, direction: Direction::Left, status, error: error.map(Into::into), at };
        assert_eq!(remote_keyboard_status(true, "kbd", "k", Some(&last(true, Some(200), None))),
            ("On", Tone::Ok, "Sent left at 14:02:03 (HTTP 200).".into()));
        assert_eq!(remote_keyboard_status(true, "kbd", "k", Some(&last(false, Some(401), None))).2,
            "Last press failed at 14:02:03: HTTP 401.");
        assert_eq!(remote_keyboard_status(true, "kbd", "k", Some(&last(false, None, Some("timeout")))).2,
            "Last press failed at 14:02:03: timeout.");
    }

    // ---- settings-status.test.js: artnetStatus, isRelayUrl

    const ON: &str = "https://relay.lan";
    fn counts(sent: u64, failures: u32, last_error: Option<&str>) -> Option<ArtnetCounts<'_>> {
        Some(ArtnetCounts { sent, failures, last_error })
    }

    #[test]
    fn artnet_is_off_while_disabled() {
        assert_eq!(artnet_status(false, ON, None).0, "Off");
    }

    #[test]
    fn artnet_warns_when_on_with_no_url_or_one_that_is_not_http() {
        for url in ["", "ftp://x", "relay"] {
            assert_eq!(artnet_status(true, url, None).1, Tone::Warn, "{url}");
        }
    }

    #[test]
    fn artnet_surfaces_the_clients_last_error() {
        let (_, tone, line) = artnet_status(true, ON, counts(3, 2, Some("HTTP 502")));
        assert_eq!(tone, Tone::Warn);
        assert!(line.contains("HTTP 502"), "{line}");
    }

    #[test]
    fn artnet_is_green_once_sending_and_ready_before_that() {
        assert_eq!(artnet_status(true, ON, counts(1, 0, None)).2, "Sending. 1 update so far.");
        assert_eq!(artnet_status(true, ON, counts(4, 0, None)).2, "Sending. 4 updates so far.");
        assert_eq!(artnet_status(true, ON, counts(0, 0, None)).1, Tone::Ok);
        assert_eq!(artnet_status(true, ON, None).1, Tone::Ok);
    }

    #[test]
    fn relay_urls_are_http_or_https_with_a_host() {
        assert!(is_relay_url("http://relay.lan:8080"));
        assert!(is_relay_url("https://spacelights.lab271.io"));
        assert!(is_relay_url("HTTP://127.0.0.1:8765/"));
        assert!(!is_relay_url("file:///etc/passwd"));
        assert!(!is_relay_url("not a url"));
        assert!(!is_relay_url("http://"));
        assert!(!is_relay_url("http://:8000"));
    }

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
