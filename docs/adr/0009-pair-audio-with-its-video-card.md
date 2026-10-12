# 0009. Pair audio with its video card; missing audio never blocks video

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-09 (#151, PR #169) |
| **Area** | Capture |
| **Evidence** | `renderer.js:836-866`; `mock-capture.js:44-47` |
| **egui verdict** | Adapt |
| **egui status** | Not started |

## Context

Audio used to be requested with the **video** device id. That always failed
with OverconstrainedError, so every input switch cost two `getUserMedia`
calls and logged a misleading error (#151).

## Decision

- Find the `audioinput` with the same `groupId` as the video device; request
  audio only if one exists.
- If the combined open fails, open video alone: "video alone is still worth
  having".
- Mock inputs get a **distinct** `groupId` each, so mock mode cannot make a
  pairing bug look correct.

## Consequences

One `getUserMedia` call in the common case. The retry path in
[0007](0007-capture-at-native-resolution.md) still has the old bug.

## Porting to egui

**Adapt.** `groupId` is WebRTC's. Natively, pair through the device's
container id (Windows `DEVPKEY_Device_ContainerId`; the Elgato exposes a
matching audio endpoint) and CoreAudio's equivalent on macOS. Keep the two
rules: audio follows its card, and a missing or failing audio device never
blocks video.

Prototype: no audio at all. With one session per device
([0008](0008-one-capture-session-per-device.md)) the "plays twice" question
answers itself: one audio stream per device, mixed per side by volume
([0010](0010-volume-belongs-to-a-side.md)).
