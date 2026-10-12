# 0019. One no-signal state machine per side, with a downtime clock

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-21 (#154, PR #270); #248 (2026-08-19) |
| **Area** | Signal |
| **Evidence** | `renderer.js:1663-1702` (`showNoSignal` / `hideNoSignal`), `:1510-1660` (boards, `formatDowntime`, `boardShouldRun`), `:1718-1757` (screensaver trigger) |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

Many paths can make a side go dark: a detection verdict, a failed open, a
disabled input, a missing device, a failed enumeration. Each used to manage
the overlay itself.

## Decision

- `showNoSignal(side)` is idempotent and stamps `noSignalSince` only on the
  transition, so a mirrored twin side does not reset the clock.
  `hideNoSignal(side)` clears it and stops the board.
- **`--no-signal` is enforced in exactly one place:** `hideNoSignal()`
  returns early. Every path that would clear the overlay goes through it,
  so one guard covers all of them; pushing the check out to callers is how
  one gets missed ([0042](0042-test-mode-flags.md)).
- The board shows `<INPUT NAME> / NO SIGNAL / DOWN mm:ss` -- `DOWN hH mmM`
  past an hour, `DOWN dD hhH` past a day -- refreshed every 5 s.
- The board runs only while its side is dark and no screensaver covers it.
- The screensaver starts after 5 min with every visible side dark
  ([0023](0023-start-after-5-min-rotate-every-10.md)).

## Consequences

A disabled input and a failed open show the same board.

## Porting to egui

**Keep.** Prototype: `side_no_signal(i)` is the single seam -- forced, no
input, a missing or disabled device, or a device that cannot be opened --
and `board_since` is the per-side clock. As in Electron, a dark, stalled or
frameless stream does **not** put the board up; it is reopened and stays
black meanwhile. Only reference detection
([0014](0014-detect-no-signal-against-references.md)) turns a live-but-dark
card into the board -- so without it, a wall whose cards are dark at night
shows black, never the board or a screensaver. That is today's Electron
behaviour on the wall too, and a reason to settle D1 in
[open-questions.md](open-questions.md).
Missing: the DOWN line, and keeping the clock across a twin side (moot with
one session per device). The board text is "NO SIGNAL / <label>" without the
downtime.
