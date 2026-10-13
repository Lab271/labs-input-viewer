// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! Which audio input belongs to which video device (ADR 0009).
//!
//! Electron paired by WebRTC's `groupId`, which Chromium derives from the
//! devices' shared container. Nothing here has a groupId: nokhwa gives a video
//! device a name and a path, cpal gives an audio endpoint a name and an id, and
//! the two ids have nothing in common. So the pairing is by **name**, which is
//! what a capture card's driver keeps in common between its two halves:
//!
//! | Platform | Video (nokhwa) | Audio (cpal) |
//! |---|---|---|
//! | Windows | `Game Capture 4K60 Pro MK.2` | `Digital Audio Interface (Game Capture 4K60 Pro MK.2)` -- WASAPI's "endpoint (adapter)" |
//! | Windows, second card | `Game Capture 4K60 Pro MK.2` | `Digital Audio Interface (2- Game Capture 4K60 Pro MK.2)` |
//! | macOS | `Cam Link 4K` | `Cam Link 4K` |
//!
//! The rule, in order:
//!
//! 1. Names are compared as **cores**: lowercased, whitespace collapsed, a
//!    Windows `N- ` duplicate prefix or a `#N` suffix taken off as the
//!    instance number, and a trailing `audio` / `video` word dropped. An audio
//!    name of the WASAPI shape `endpoint (adapter)` offers its adapter as a
//!    core as well as the whole name.
//! 2. Equal cores pair. Where several devices share a core -- the wall's two
//!    identical cards -- the Nth video device pairs with the Nth audio device,
//!    N being the explicit instance number if the name carries one, else its
//!    place among the unnumbered ones of that name (Windows leaves the first
//!    unnumbered). **This is the one guess in the rule** and it is
//!    logged; see "Checking on the wall" in ADR 0009.
//! 3. Failing that, a core whose words all appear, in order, in the other
//!    (at least two words) pairs, so `Elgato Game Capture HD60 S+` finds
//!    `Game Capture HD60 S+`. A built-in microphone never matches a webcam
//!    this way: `MacBook Pro Microphone` shares no core with `FaceTime HD
//!    Camera`.
//! 4. Each audio device pairs at most once, and an operator override
//!    (`inputs.<id>.audioDevice` in settings.json, a case-insensitive
//!    substring of the audio name, or `none`) beats all of the above -- the
//!    escape hatch if rule 2 guesses the two cards the wrong way round.
//!
//! A video device with no pair simply has no audio: "missing audio never
//! blocks video".

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioInput {
    /// cpal's `DeviceId` as a string (`wasapi:{0.0.1.00000000}.{guid}`), or
    /// `mock-audio-N`.
    pub id: String,
    pub name: String,
}

/// A name reduced for comparison, and which instance of that name it is.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Core {
    words: Vec<String>,
    /// Explicit instance from `2- ` or `#2`; None means "by order".
    instance: Option<u32>,
}

fn core(name: &str) -> Option<Core> {
    let mut s = name.trim().to_lowercase();
    let mut instance = None;
    // Windows numbers duplicate device names "2- Name".
    if let Some((n, rest)) = s.split_once("- ")
        && !n.is_empty()
        && n.chars().all(|c| c.is_ascii_digit())
    {
        instance = n.parse().ok();
        s = rest.to_owned();
    }
    // ...and some drivers "Name #2".
    if let Some((rest, n)) = s.rsplit_once(" #")
        && !n.is_empty()
        && n.chars().all(|c| c.is_ascii_digit())
    {
        instance = n.parse().ok();
        s = rest.to_owned();
    }
    let mut words: Vec<String> = s.split_whitespace().map(str::to_owned).collect();
    while words.len() > 1 && matches!(words.last().map(String::as_str), Some("audio" | "video")) {
        words.pop();
    }
    (!words.is_empty()).then_some(Core { words, instance })
}

/// `endpoint (adapter)` -> `(endpoint, adapter)`; the last balanced
/// parenthesised group, so `Microphone (Realtek(R) Audio)` gives
/// `Realtek(R) Audio`.
fn split_adapter(name: &str) -> Option<(&str, &str)> {
    let name = name.trim_end();
    let body = name.strip_suffix(')')?;
    let mut depth = 0;
    for (i, c) in body.char_indices().rev() {
        match c {
            ')' => depth += 1,
            '(' if depth == 0 => return Some((body[..i].trim_end(), &body[i + 1..])),
            '(' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// The cores an audio name offers: its adapter, if it has one, then the
/// whole name.
fn audio_cores(name: &str) -> Vec<Core> {
    let mut cores = Vec::new();
    if let Some((_, adapter)) = split_adapter(name) {
        cores.extend(core(adapter));
    }
    if let Some(c) = core(name)
        && !cores.contains(&c)
    {
        cores.push(c);
    }
    cores
}

/// Which instance of its name a device is: its own number if it has one,
/// else its place among the unnumbered ones. Windows leaves the first of a
/// name unnumbered and calls the next `2- `, so the unnumbered one is 1.
fn ordinal(seen: &mut HashMap<Vec<String>, u32>, c: &Core) -> u32 {
    c.instance.unwrap_or_else(|| {
        let n = seen.entry(c.words.clone()).or_insert(0);
        *n += 1;
        *n
    })
}

/// Do `short`'s words all appear, in order and contiguously, in `long`?
fn contains_words(long: &[String], short: &[String]) -> bool {
    short.len() >= 2 && short.len() <= long.len() && long.windows(short.len()).any(|w| w == short)
}

/// Electron's `[Audio]` log names a device by its label; this explains a pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// Set in settings.json.
    Override,
    /// Same name, and the same instance by number or by order.
    Name,
    /// Same name, but more video devices of that name than audio ones (or the
    /// other way round), so the instance could not be matched.
    NameAnyInstance,
    /// One name's words appear in the other.
    Words,
}

/// Pair each video device with an audio input. `videos` is `(id, label)` in
/// enumeration order; `overrides` maps a video id to an `audioDevice` setting.
/// Returns video id -> (index into `audio`, why).
pub fn pair(
    videos: &[(&str, &str)],
    audio: &[AudioInput],
    overrides: &HashMap<String, String>,
) -> HashMap<String, (usize, Why)> {
    let mut result = HashMap::new();
    let mut taken = vec![false; audio.len()];
    let mut done = vec![false; videos.len()];

    // Rule 4 first: an override is the operator's word.
    for (v, (id, _)) in videos.iter().enumerate() {
        let Some(want) = overrides.get(*id).map(|w| w.trim().to_lowercase()) else { continue };
        done[v] = true;
        if want.is_empty() || want == "none" {
            continue;
        }
        if let Some(a) = (0..audio.len()).find(|&a| !taken[a] && audio[a].name.to_lowercase().contains(&want)) {
            taken[a] = true;
            result.insert((*id).to_owned(), (a, Why::Override));
        }
    }

    // Ordinals: the Nth device sharing a core, counting from 1, unless the
    // name numbers itself.
    let video_cores: Vec<Option<(Core, u32)>> = {
        let mut seen: HashMap<Vec<String>, u32> = HashMap::new();
        videos.iter().map(|(_, label)| core(label).map(|c| {
            let ord = ordinal(&mut seen, &c);
            (c, ord)
        })).collect()
    };
    let audio_cores: Vec<Vec<(Core, u32)>> = {
        let mut seen: HashMap<Vec<String>, u32> = HashMap::new();
        audio.iter().map(|a| audio_cores(&a.name).into_iter().map(|c| {
            let ord = ordinal(&mut seen, &c);
            (c, ord)
        }).collect()).collect()
    };

    // Score every pair, then take the best first: 3 same name and instance,
    // 2 same name, 1 words contained.
    let mut scored = Vec::new();
    for (v, vc) in video_cores.iter().enumerate() {
        let Some((vc, vord)) = vc else { continue };
        if done[v] {
            continue;
        }
        for (a, cores) in audio_cores.iter().enumerate() {
            let score = cores.iter().map(|(ac, aord)| {
                if ac.words == vc.words {
                    if aord == vord { 3 } else { 2 }
                } else if contains_words(&ac.words, &vc.words) || contains_words(&vc.words, &ac.words) {
                    1
                } else {
                    0
                }
            }).max().unwrap_or(0);
            if score > 0 {
                scored.push((score, v, a));
            }
        }
    }
    // Highest score first; ties in enumeration order, so the result is
    // deterministic.
    scored.sort_by(|x, y| y.0.cmp(&x.0).then(x.1.cmp(&y.1)).then(x.2.cmp(&y.2)));
    for (score, v, a) in scored {
        if done[v] || taken[a] {
            continue;
        }
        done[v] = true;
        taken[a] = true;
        let why = match score {
            3 => Why::Name,
            2 => Why::NameAnyInstance,
            _ => Why::Words,
        };
        result.insert(videos[v].0.to_owned(), (a, why));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audio(names: &[&str]) -> Vec<AudioInput> {
        names.iter().enumerate().map(|(i, n)| AudioInput { id: format!("a{i}"), name: (*n).to_owned() }).collect()
    }

    /// video id -> audio name, for readable assertions.
    fn run(videos: &[(&str, &str)], names: &[&str], overrides: &[(&str, &str)]) -> HashMap<String, String> {
        let a = audio(names);
        let o = overrides.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        pair(videos, &a, &o).into_iter().map(|(v, (i, _))| (v, a[i].name.clone())).collect()
    }

    /// The wall: two identical Elgato cards, as Windows names them, with the
    /// rest of a PC's capture endpoints around them.
    const WALL_AUDIO: &[&str] = &[
        "Microphone (Realtek(R) Audio)",
        "Digital Audio Interface (Game Capture 4K60 Pro MK.2)",
        "Digital Audio Interface (2- Game Capture 4K60 Pro MK.2)",
        "Stereo Mix (Realtek(R) Audio)",
    ];

    #[test]
    fn the_walls_two_cards_pair_in_order() {
        let videos = [("v1", "Game Capture 4K60 Pro MK.2"), ("v2", "Game Capture 4K60 Pro MK.2")];
        let p = run(&videos, WALL_AUDIO, &[]);
        assert_eq!(p["v1"], "Digital Audio Interface (Game Capture 4K60 Pro MK.2)");
        assert_eq!(p["v2"], "Digital Audio Interface (2- Game Capture 4K60 Pro MK.2)");
        assert_eq!(p.len(), 2, "the Realtek endpoints pair with nothing");
    }

    #[test]
    fn an_explicit_instance_number_beats_enumeration_order() {
        // Audio enumerated second card first.
        let names = ["Digital Audio Interface (2- Game Capture 4K60 Pro MK.2)", "Digital Audio Interface (Game Capture 4K60 Pro MK.2)"];
        let videos = [("v1", "Game Capture 4K60 Pro MK.2"), ("v2", "2- Game Capture 4K60 Pro MK.2")];
        let p = run(&videos, &names, &[]);
        assert_eq!(p["v1"], names[1]);
        assert_eq!(p["v2"], names[0]);
    }

    #[test]
    fn audio_and_video_suffixes_are_ignored() {
        let p = run(&[("v", "Game Capture 4K60 Pro MK.2 Video")], &["Game Capture 4K60 Pro MK.2 Audio"], &[]);
        assert_eq!(p["v"], "Game Capture 4K60 Pro MK.2 Audio");
    }

    #[test]
    fn mac_names_are_the_same_on_both_sides() {
        let names = ["MacBook Pro Microphone", "Cam Link 4K", "Logitech BRIO", "BlackHole 2ch"];
        let videos = [("cam", "FaceTime HD Camera"), ("link", "Cam Link 4K"), ("brio", "Logitech BRIO")];
        let p = run(&videos, &names, &[]);
        assert_eq!(p.get("cam"), None, "the built-in mic is not the camera's audio");
        assert_eq!(p["link"], "Cam Link 4K");
        assert_eq!(p["brio"], "Logitech BRIO");
    }

    #[test]
    fn a_vendor_prefix_still_pairs_by_words() {
        let p = run(&[("v", "Elgato Game Capture HD60 S+")], &["Digital Audio Interface (Game Capture HD60 S+)"], &[]);
        assert_eq!(p["v"], "Digital Audio Interface (Game Capture HD60 S+)");
    }

    #[test]
    fn single_words_and_numbers_do_not_cross_match() {
        // "Mock Input 1" must not take "Mock Input 10"'s audio, and one shared
        // word ("USB") is not a match.
        let p = run(&[("v", "Mock Input 1"), ("u", "USB Camera")], &["Digital Audio Interface (Mock Input 10)", "Microphone (USB Audio Device)"], &[]);
        assert!(p.is_empty(), "{p:?}");
    }

    #[test]
    fn mock_inputs_each_find_their_own() {
        let videos = [("mock-input-1", "Mock Input 1"), ("mock-input-2", "Mock Input 2")];
        let names = ["Digital Audio Interface (Mock Input 2)", "Digital Audio Interface (Mock Input 1)"];
        let p = run(&videos, &names, &[]);
        assert_eq!(p["mock-input-1"], names[1]);
        assert_eq!(p["mock-input-2"], names[0]);
    }

    #[test]
    fn each_audio_device_pairs_once() {
        // Three cards, two audio endpoints: the third has no audio.
        let videos = [("a", "Cam Link 4K"), ("b", "Cam Link 4K"), ("c", "Cam Link 4K")];
        let p = run(&videos, &["Cam Link 4K", "Cam Link 4K"], &[]);
        assert_eq!(p.len(), 2);
        assert!(p.contains_key("a") && p.contains_key("b"));
    }

    #[test]
    fn an_override_swaps_or_silences() {
        let videos = [("v1", "Game Capture 4K60 Pro MK.2"), ("v2", "Game Capture 4K60 Pro MK.2")];
        let p = run(&videos, WALL_AUDIO, &[("v1", "2- game capture")]);
        assert_eq!(p["v1"], WALL_AUDIO[2]);
        assert_eq!(p["v2"], WALL_AUDIO[1], "the other card takes what is left");
        let p = run(&videos, WALL_AUDIO, &[("v1", "none")]);
        assert_eq!(p.get("v1"), None);
        assert!(p.contains_key("v2"));
    }

    #[test]
    fn adapter_names_parse() {
        assert_eq!(split_adapter("Microphone (Realtek(R) Audio)"), Some(("Microphone", "Realtek(R) Audio")));
        assert_eq!(split_adapter("Cam Link 4K"), None);
        assert_eq!(core("2- Game Capture 4K60 Pro MK.2").unwrap().instance, Some(2));
        assert_eq!(core("Cam Link 4K #3").unwrap().instance, Some(3));
        assert_eq!(core("Audio"), Some(Core { words: vec!["audio".into()], instance: None }), "a lone word is kept");
    }
}
