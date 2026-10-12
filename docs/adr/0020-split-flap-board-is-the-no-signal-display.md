# 0020. The split-flap board is the no-signal display, not a screensaver

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-08 (#147); moved out of rotation 2026-08-09 (PR #148, #92); live rows 2026-08-21 (#154, PR #270) |
| **Area** | Screensavers |
| **Evidence** | `screensavers/split-flap.js:1-20, 105-117, 224-300`; `renderer.js:51-53, 1520-1620`; `index.html` fallback text |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

#92 asked for a no-signal screen that "reads as intentional rather than as a
crashed app". The board was first built as screensaver number 22, which
meant it appeared only after five minutes of darkness -- the opposite of the
intent: an abstract animation the instant a feed dies reads as a crash.

A random palette made it green or magenta. "NO SIGNAL" was clipped to "NO
SIGNA" at 16:9. Two hidden boards were found still rendering behind an
opaque screensaver.

## Decision

- One board per dark half, shown immediately; removed from the screensaver
  list.
- Fixed cream-on-near-black, by construction.
- Columns: at least the longest built-in line + 2. Live text truncates with
  a trailing "." -- 16 columns per half, 22 for the whole wall.
- Live rows are pushed through `setMessages()` rather than passing app state
  into the saver (#154 options 1 and 2 rejected). A malformed push restores
  the built-in list.
- An HTML "NO SIGNAL / Connect your device to the presenter booth" stays as
  the fallback if WebGL fails, hidden only once the board runs.
- Boards stop while a screensaver covers them.

## Consequences

The board is information, not decoration: what is dark, and for how long.

## Porting to egui

**Keep.** Prototype: a painter-drawn board (`paint_split_flap`, no GL), one
per dark half, flipping into place, cream and amber rows. It needs the DOWN
row ([0019](0019-one-no-signal-state-machine.md)), the column rules, and the
"Connect your device to the presenter booth" line. Porting the GL board
itself needs a glyph atlas from a Rust font rasteriser
([0029](0029-text-through-a-baked-glyph-atlas.md)); the painter version may
simply be good enough.
