# 0010. Volume belongs to a side; the OS volume is shown and set directly

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-01-24 (#25, 6caf489); 2026-08-25 (#283) |
| **Area** | Capture |
| **Evidence** | `renderer.js:1329-1404` (gain node per side), `:1436-1489` (polling), `buildPickerVolume` :2885; `src/main/index.js:682-795`; USER_GUIDE |
| **egui verdict** | Adapt |
| **egui status** | Partial |

## Context

Each half has a volume slider in its picker, and the capsule has an output
volume control. The OS volume was read by shelling out: AppleScript on
macOS, PowerShell compiling an inline C# `IAudioEndpointVolume` interop on
Windows, every 2 s for the app's life. PID churn on the wall confirmed it
(#283).

## Decision

- `<video>` elements are muted; audio goes MediaStreamSource -> GainNode ->
  destination, one per side. Volume 0..1 per side, persisted as
  `leftVolume` / `rightVolume`. It belongs to the half, "so it stays put
  when you switch inputs" (USER_GUIDE). A both-halves picker sets both.
- OS master volume is polled every 2 s **only while the dropdown is open**,
  once at startup, with a 30 s safety stop (#283).

## Consequences

The volume UI is accurate only while visible, which is the only time it
matters.

## Porting to egui

**Adapt.** Keep per-side volume and the OS volume control as product
behaviour. Replace the shell-out with WASAPI (`windows` crate) and CoreAudio
calls; the polling concern disappears, since a native read costs
microseconds. Keep `leftVolume` / `rightVolume` and `systemVolume` in
`settings.json`.

Prototype: **partial** (`input_viewer_egui/src/audio.rs`).

- Each wall picker has its half's volume slider, a "Both halves" picker
  sets both, and the capsule has Output volume, as dropdown 2b
  ([0033](0033-controls-laid-over-the-wall.md)). The speaker button mutes
  by setting 0 and restores the previous level; there is no separate mute
  state. Persisted as `leftVolume` / `rightVolume` (0..1, default 1, the
  renderer's defaults since main sets none, E7).
- **Gain rule.** One capture per device, not per side, so a device's gain
  is the **largest** volume among the audible halves showing it, times the
  output volume. Dual: both halves audible; single: only the left. Max, not
  sum: a sum is E12's double level; not average: turning one copy down must
  not quiet the other half. All audible devices are mixed into the default
  output device (which cpal follows when the default changes).
- **Not ported: the OS master volume.** Output volume is the app's own
  master gain, `outputVolume` (0..1, default 1). Electron's `systemVolume`
  mirrored the OS level; reading it as an app gain would attenuate twice on
  an import, so it is kept untouched in settings.json. Setting the OS
  volume natively (WASAPI `IAudioEndpointVolume`, CoreAudio) is still open.
- The mixed level is logged (`[Audio] output rms ... dBFS`) every 5 s in
  mock and `--hud` runs and every 10 min otherwise, so a gain can be
  checked from the log.
