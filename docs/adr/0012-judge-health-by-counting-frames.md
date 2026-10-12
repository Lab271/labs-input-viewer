# 0012. Judge capture health by counting frames, never by measuring motion

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-01 (PR #311); motion rejected 2026-08-09 (#159) |
| **Area** | Signal |
| **Evidence** | `stream-health.js` (constants :40-57), `renderer.js:989-1263`; #311, #159, #314 |
| **egui verdict** | Keep |
| **egui status** | Done |

## Context

A card sometimes boots showing black with a live source plugged in. On
2026-10-01 the app started at 12:12:27 and someone restarted it by hand at
12:22:35. The split-flap board never appeared: **the wall has no no-signal
references** ([0014](0014-detect-no-signal-against-references.md)), and the
stream had opened fine. The Elgato driver exposes no signal-lock state
(registry and services checked), so the picture is the only evidence.

Motion detection was rejected in #159: "the wall shows presentations, so a
held slide IS a frozen feed."

## Decision

Sample every 2000 ms: a frame counter (`track.stats.totalFrames`, falling
back to `getVideoPlaybackQuality`) and the luma mean and spread of a
thumbnail. Statuses:

| Status | Rule |
|---|---|
| `opening` | not yet decided |
| `ok` | frames arriving |
| `no-frames` | no frame within 8000 ms of opening |
| `stalled` | counter stopped for 6000 ms, **only if the feed had reached >= 5 fps first** |
| `ended` | track ended |
| `open-failed` | open rejected |
| `dark` | luma spread <= 3 for 6000 ms |

A counter going backwards is a new baseline. "Was flowing first" keeps a
static virtual camera, which really sends one frame, from being reopened in
a loop.

## Consequences

This, not reference detection, is what keeps the wall working. The dark
check uses the 384x216 crop thumbnail since #314; comments and CLAUDE.md
still say 32x18 ([open questions](open-questions.md)).

## Porting to egui

**Keep** the statuses, thresholds and the no-motion rule. Natively the frame
counter is the capture callback itself, which is simpler than Chromium's
stats.

**Done.** `health.rs` is a line-for-line port of `stream-health.js` --
constants, statuses, backoffs -- with its unit tests ported case for case.
Sampled every 2 s per device from the frame counter and the luma of a
**box-averaged** 384x216 thumbnail (averaged as Electron's canvas downscale
is, so a card's noisy no-signal screen still reads flat). A stream error is
acted on at once, as a rejected open or an ended track is in Electron.
Verified end to end with `--mock-fault=dark|stall|fail`.
