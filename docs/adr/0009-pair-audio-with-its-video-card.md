# 0009. Pair audio with its video card; missing audio never blocks video

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-09 (#151, PR #169) |
| **Area** | Capture |
| **Evidence** | `renderer.js:836-866`; `mock-capture.js:44-47` |
| **egui verdict** | Adapt |
| **egui status** | Partial |

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

Prototype: **partial** -- ported with `cpal` (WASAPI / CoreAudio) in
`input_viewer_egui/src/audio.rs`, but paired by **name**, not container id,
and not yet tried on the wall.

- **Pairing** (`audio/pairing.rs`, unit tested): names compared lowercased,
  with a Windows `2- ` prefix or `#2` suffix read as the instance and a
  trailing `audio`/`video` dropped; a WASAPI name `endpoint (adapter)` is
  matched on its adapter. Equal names pair, the Nth of a name with the Nth;
  failing that, a name whose words all appear in the other. Each audio device
  pairs once. `inputs.<id>.audioDevice` in settings.json (part of the audio
  name, or `none`) overrides it. Container ids were not used: neither nokhwa
  nor cpal exposes them, and reading them means Windows-only property-store
  code that could not be tested here.
- **The two rules hold:** audio follows its card, and a capture that fails,
  is missing or vanishes is logged, retried every 5 s, and never touches the
  video stream (separate threads, no shared locks, bounded rings).
- **One capture per device**, keyed like the video slot, so E12 cannot
  happen ([0008](0008-one-capture-session-per-device.md)).
- **Mock inputs** each get a distinct synthetic device and tone, named the
  WASAPI way (`Digital Audio Interface (Mock Input N)`), so they go through
  the same pairing rule -- the counterpart of the distinct mock `groupId`.

**Checking on the wall.** The two cards have identical names, so which audio
endpoint belongs to which card is the one guess (by instance number, else
order). The log names every pairing on startup and on every device change:
`[Audio] "Game Capture 4K60 Pro MK.2" (1a2b3c4d) pairs with "..." (same
name)`. Put a different source on each card, put each on its own half, and
turn one half's volume down: the right one must go quiet. If they are
crossed, set `audioDevice` on one input. Also confirm the real endpoint names
(`[Audio] inputs: ...`) match the shapes in `pairing.rs`'s tests.
