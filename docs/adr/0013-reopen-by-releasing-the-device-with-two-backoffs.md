# 0013. Reopen a sick device by releasing it fully, on separate backoffs for faults and for dark

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-01 (PR #311) |
| **Area** | Signal |
| **Evidence** | `stream-health.js:52-54, 238-262`; `renderer.js:1266-1297` (`reopenDevice`), `REOPEN_RELEASE_MS` :995, `:1183-1203` |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

The startup script cannot restart the cards (`pnputil` returns 3010 because
`audiodg.exe` holds the audio endpoint), so recovery is the app's job
([0006](0006-app-and-wall-setup-live-in-separate-repos.md)). Reopening a
flat feed is safe because its blank moment cannot be seen. "Nothing plugged
in all night" is the steady state, so dark must not be retried fast.

## Decision

- **Faults** (`open-failed`, `ended`, `no-frames`, `stalled`): reopen after
  2 s, 5 s, 15 s, 30 s, then every 60 s.
- **Dark:** 5 s, 15 s, 30 s, 60 s, 5 min, then every 10 min.
- The attempt count resets after 60 s healthy. Delays run from the last
  reopen or from the open.
- A reopen stops **every side** using the device, closes all frame sources,
  waits 500 ms "so the driver sees the device closed rather than a
  handover", then reopens each side unless the operator switched it
  meanwhile. One reopen in flight per device.
- No devices at all: re-enumerate at 5 s, 10 s, then every 30 s.
- Not started under `--no-signal`.

## Consequences

A reopen fixes card and driver-session hangs. It does not fix a dead HDMI or
HDBaseT link ("Input Viewer reopening the card: no", `edid/README.md`).
Whether it ever cured the original "boots dark" symptom is not recorded.

## Porting to egui

**Keep** both schedules, the 60 s reset, the 500 ms release gap and
one-in-flight. "Release every side" becomes implicit with one session per
device ([0008](0008-one-capture-session-per-device.md)).

Prototype (`app.rs` `REOPEN_BACKOFF`): the fault schedule is right, but it
resets on the first `Ok` rather than after 60 s healthy, never reopens on
dark, and reopens without the 500 ms gap (the old stream's thread is joined
in the background while the new one opens).
