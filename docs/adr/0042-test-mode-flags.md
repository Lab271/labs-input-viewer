# 0042. Test-mode flags reach every state without hardware, and say so loudly

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-19 (#248, PR #259, da78a2c) |
| **Area** | Platform |
| **Evidence** | `test-flags.js`; `mock-capture.js`; `renderer.js:620-651, 655-678, 799-830, 1680-1686, 1299-1305, 4694-4703`; `src/main/index.js:14-41, 575-579` |
| **egui verdict** | Keep |
| **egui status** | Done |

## Context

Reaching the no-signal board and the screensavers meant unplugging a capture
card. And "a flag that silently does nothing means someone at the wall is
looking at a production-mode screen believing it is in test mode."

## Decision

| Flag | Effect |
|---|---|
| `--mock[=N]` | N synthetic inputs (1-8, default 4). No hardware, no camera prompt. |
| `--no-signal` | Every input pinned to no-signal: the board from launch. |
| `--screensaver-delay=MS` | Replaces the 5-minute wait; 0 starts a saver at once. |

- Orthogonal on purpose: `--no-signal` alone is the fast path to the board;
  with `--mock` there are named inputs to switch between while dark.
- **Mock inputs are real streams** (`canvas.captureStream(15)`, 1280x720,
  colour bars and a sweep), so detection cannot tell them from a card;
  distinct `groupId` each ([0009](0009-pair-audio-with-its-video-card.md)).
- **`--mock` never writes settings** ([0041](0041-settings-json.md)).
- **`--no-signal` is enforced only in `hideNoSignal()`**
  ([0019](0019-one-no-signal-state-machine.md)), and detection and health
  are not started at all, rather than run and discarded.
- Unknown flags that look like ours are logged as errors, never fatal. Main
  forwards candidates with a deliberately looser filter than the parser's
  list (`^--(mock|no.?signal|screensaver)`), so a typo like `--nosignal`
  reaches the parser and is reported.

## Porting to egui

**Keep.** Prototype: done in `flags.rs` (tested), with the same names and
semantics; unrecognised arguments are logged; mock frames go through the
same capture path as a camera; the single seam is `side_no_signal()`. The
main/renderer forwarding split disappears. The prototype adds three
dev-only flags -- `--screensaver=NAME`, `--test-pattern[=WxH@FPS]` and
`--hud` -- documented in its README.
