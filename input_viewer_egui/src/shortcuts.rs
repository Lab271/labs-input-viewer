// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271
//! The keyboard shortcuts, as data -- a port of `shortcuts.js` (#258).
//!
//! One list feeds both the key handler and the legend, so the two cannot drift.
//! The remote-keyboard rows are omitted: the prototype has no remote keyboard.

use eframe::egui::Key;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    SelectInput(usize),
    LayoutDual,
    LayoutSingle,
    Freeze,
    ScreensaverToggle,
    ScreensaverNext,
    ScreensaverPrev,
    Fullscreen,
    Escape,
    Quit,
}

pub struct Shortcut {
    /// Each key with the action it runs. Most rows bind one action; `1`-`4`
    /// bind a different input each.
    pub keys: &'static [(Key, Action)],
    pub chips: &'static str,
    pub label: &'static str,
}

use Action::*;

pub const SHORTCUTS: &[Shortcut] = &[
    Shortcut {
        keys: &[(Key::Num1, SelectInput(0)), (Key::Num2, SelectInput(1)), (Key::Num3, SelectInput(2)), (Key::Num4, SelectInput(3))],
        chips: "1–4",
        label: "Select input",
    },
    Shortcut { keys: &[(Key::D, LayoutDual)], chips: "D", label: "Dual view" },
    Shortcut { keys: &[(Key::S, LayoutSingle)], chips: "S", label: "Single view" },
    Shortcut { keys: &[(Key::Space, Freeze)], chips: "Space", label: "Freeze frame" },
    Shortcut { keys: &[(Key::V, ScreensaverToggle)], chips: "V", label: "Screensaver on demand" },
    // '=' is the unshifted key that produces '+' on US/UK layouts.
    Shortcut { keys: &[(Key::Plus, ScreensaverNext), (Key::Equals, ScreensaverNext)], chips: "+", label: "Next screensaver" },
    Shortcut { keys: &[(Key::Minus, ScreensaverPrev)], chips: "-", label: "Previous screensaver" },
    Shortcut { keys: &[(Key::F, Fullscreen), (Key::F11, Fullscreen)], chips: "F / F11", label: "Fullscreen" },
    Shortcut { keys: &[(Key::Escape, Escape)], chips: "Esc", label: "Exit fullscreen, unfreeze, close panels" },
    Shortcut { keys: &[(Key::Q, Quit)], chips: "Q", label: "Quit" },
];

/// The action bound to `key`, looked up in the table above.
pub fn action_for(key: Key) -> Option<Action> {
    SHORTCUTS.iter().flat_map(|s| s.keys).find(|(k, _)| *k == key).map(|(_, a)| *a)
}

/// The chip for the input at `index`, or None past the fourth: labelling a fifth
/// row `5` would promise a binding that does not exist.
pub fn input_key_for(index: usize) -> Option<&'static str> {
    ["1", "2", "3", "4"].get(index).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn no_key_is_claimed_twice() {
        // A duplicate would silently shadow the later entry in action_for.
        let mut seen = HashSet::new();
        for (k, _) in SHORTCUTS.iter().flat_map(|s| s.keys) {
            assert!(seen.insert(*k), "{k:?} listed twice");
        }
    }

    #[test]
    fn every_action_is_reachable_from_a_key() {
        let all = [LayoutDual, LayoutSingle, Freeze, ScreensaverToggle, ScreensaverNext,
            ScreensaverPrev, Fullscreen, Escape, Quit, SelectInput(0), SelectInput(3)];
        for a in all {
            assert!(SHORTCUTS.iter().flat_map(|s| s.keys).any(|(_, b)| *b == a), "{a:?} has no key");
        }
    }

    #[test]
    fn number_chips_stop_at_four() {
        assert_eq!(input_key_for(3), Some("4"));
        assert_eq!(input_key_for(4), None);
    }
}
