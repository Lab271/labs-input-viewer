# 0010. Volume belongs to a side; the OS volume is shown and set directly

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-01-24 (#25, 6caf489); 2026-08-25 (#283) |
| **Area** | Capture |
| **Evidence** | `renderer.js:1329-1404` (gain node per side), `:1436-1489` (polling), `buildPickerVolume` :2885; `src/main/index.js:682-795`; USER_GUIDE |
| **egui verdict** | Adapt |
| **egui status** | Not started |

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
`settings.json`. Not started in the prototype.
