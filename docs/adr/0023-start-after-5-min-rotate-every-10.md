# 0023. Start a screensaver after 5 minutes dark, rotate every 10, never repeat the last

| | |
|---|---|
| **Status** | Accepted -- dismissal is Open |
| **Date** | 2026-08-06 (PR #132, rotation); 2026-08-09 (#158, stepping keys); 2026-01-24 (#25, shake to dismiss) |
| **Area** | Screensavers |
| **Evidence** | `renderer.js:144, 149` (`dvdScreensaverDelay`, `screensaverRotateDelay`), `:1765-1766` (crossfade), `:1945-1970`; `registry.js:89-92, 158-177`; USER_GUIDE |
| **egui verdict** | Keep; Decide on dismissal |
| **egui status** | Partial |

## Context

No-signal can last days. Without rotation a fresh seed is seen once per
no-signal event and then frozen for hours; a repeat pick "reads as the
rotation being broken". The 5-minute delay predates the framework; the
10-minute rotation value's rationale is not recorded.

## Decision

- Start after 5 min with every visible side dark; `--screensaver-delay`
  overrides it.
- Rotate every 10 min. `pickRandomIndex(avoid)` excludes the previous index
  and unavailable savers; if nothing is left it ignores availability ("same
  saver twice is a much smaller problem than a blank one"). No retry loop.
- Crossfade between savers: 300 ms out, 520 ms in.
- `V` toggles a saver on demand; `+` / `-` step, and stepping restarts the
  rotation countdown.
- **Dismissal:** signal returning, `V`, or **shaking the mouse** (4 reversals
  of > 10 px within 500 ms).

## Consequences

**USER_GUIDE says** "move the mouse / press any key / touch the screen" ends
the screensaver. **The code does not**: only shake, `V` or signal restore do.

## Porting to egui

**Keep** the timings and never-repeat rule. Prototype: 5 min delay, 10 min
rotation, `index_except` never repeats (tested), `V` and `+`/`-`. Missing:
the crossfade, `is_available`, and shake.

**Decide dismissal** before porting it: either the guide is right (any input
dismisses) or the code is (shake only, so a nudged mouse or a passing touch
does not drop the wall back to the board). Then fix whichever is wrong in
Electron too.
