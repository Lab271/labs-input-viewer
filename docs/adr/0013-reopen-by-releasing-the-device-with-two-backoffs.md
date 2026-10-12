# 0013. Reopen a sick device by releasing it fully, on separate backoffs for faults and for dark

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-01 (PR #311) |
| **Area** | Signal |
| **Evidence** | `stream-health.js:52-54, 238-262`; `renderer.js:1266-1297` (`reopenDevice`), `REOPEN_RELEASE_MS` :995, `:1183-1203` |
| **egui verdict** | Keep |
| **egui status** | Done |

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
one-in-flight. "Release every side" is implicit with one session per device
([0008](0008-one-capture-session-per-device.md)).

**Done.** On a reopen the slot drops its stream -- closing the device -- and
opens it again after the 500 ms gap; the health state outlives the stream,
so attempts accumulate. Measured with `--mock-fault`:

| Fault | Observed |
|---|---|
| stall | stalled 6 s after the last frame; closed; reopened 0.5 s later; recovers; repeats |
| dark | dark after 6 s flat; first reopen at once (5 s since open had passed), second 15 s after the first |
| fail | open-failed immediately; reopens at +2 s, then +5 s |

Decisions are taken at the 2 s sample, as in Electron. Not ported: the
no-devices re-enumeration at 5 s / 10 s / 30 s (the prototype polls every
10 s), and skipping the backoff when a device-change event brings a device
back ([0011](0011-choose-startup-inputs-by-precedence.md)).
