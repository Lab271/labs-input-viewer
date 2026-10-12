# 0034. One shortcut list drives the keys, the legend and the docs

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-19 (#258, PR #260); legend 2026-08-20 (PR #266); keyboard rules 2026-10-03 (PR #334); rebinding rejected 2026-08-20 (#246) |
| **Area** | UI |
| **Evidence** | `shortcuts.js` (`SHORTCUTS`, `SHORTCUTS_BY_KEY`, `inputKeyFor`); `renderer.js` `SHORTCUT_ACTIONS` :4026, `isTypingTarget` :4084, Esc :4053; `test/docs-shortcut-list.test.js`; `styles.css:1118` |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

There were four lists of shortcuts and three had drifted: the Settings table
lacked Q, V, +/- and F11; the README lacked the remote-keyboard row; the user
guide lacked F11. D and S had been documented for months but never wired up
(#157). Nothing failed; they were just quietly wrong.

## Decision

- `shortcuts.js` is data only -- id, keys, chips, label, note,
  preventDefault -- and imports nothing. Actions are keyed by id elsewhere.
  A duplicate key throws at load. A list entry with no action fails a test.
- Consumers: the key handler, the dropdown's chips, the legend; README and
  USER_GUIDE stay hand-written but a test asserts every chip appears in both.
- No chip past the fourth input: "labelling a fifth row '5' would promise a
  binding that does not exist." `=` and `_` are bound as the unshifted +/-.
- **Legend:** a dropup on the bottom edge showing **every** row -- an
  "important ones" subset would be another hand-kept list. Labels wrap, never
  truncate (at 1100 px, 3 of 12 ellipsised, including both "(if the remote
  keyboard is enabled)" caveats).
- **Keyboard rules:** shortcuts ignore every form control, not just `<input>`
  (with a `<select>` focused, Q quit the app). **Esc does one thing per
  press:** with any panel open it closes panels and stops; only with nothing
  open does it unfreeze and leave fullscreen.
- Chips have a 12 px floor: 10 px at 0.55 opacity "read fine on a laptop and
  was invisible on the wall".
- Rebindable keys: not wanted (#246).

## Consequences

The legend is the only rendered copy since #334 removed the Settings table.

## Porting to egui

**Keep.** Prototype: `shortcuts.rs` is a `const` table of `(Key, Action)`
that both the handler and the legend read; tests check for duplicate keys
and for actions with no key. Esc one-thing-per-press and the typing guard
(`egui_wants_keyboard_input()`, and everything except Esc while Settings is
open) are done. Missing: the remote-keyboard rows
([0040](0040-remote-keyboard.md)) and the README/USER_GUIDE test.
