# 0030. Drive the room's lights from the screensaver: sparse readback, 1 Hz, only when enabled

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-11 (41513aa, PR #199, #59); 2026-08-21 (49707c5, PR #275); GET snapshot 2026-08-25 (PR #293) |
| **Area** | Screensavers |
| **Evidence** | `gl-base.js:165-302, 400-457, 603-622`; `artnet-sync.js:1-150, 300-345, 440-470, 783-787`; `src/main/index.js:442-504` |
| **egui verdict** | Keep |
| **egui status** | Not started |

## Context

Art-Net fixtures in the room can follow the screensaver's colours through an
`artnet-relay` HTTP service.

- `blitFramebuffer` from WebGL's RGB8 default framebuffer to an RGBA8 target
  returns INVALID_OPERATION with LINEAR, NEAREST and no scaling alike.
- The first version read back every frame and was registered
  unconditionally. On the wall's RTX PRO 4000, two split-flap boards did 64
  `readPixels` stalls per frame and ran at **1.4 fps against 118.9**. The cost
  had been measured on Apple Silicon at 4% and dismissed (#275).
- Averaging the whole frame gives grey.

## Decision

- Read an 8x4 grid of 8x8 tiles straight from the default framebuffer, once
  per second per runtime, inside the frame. Observers register only while
  Art-Net is enabled.
- Colour is a saturation- and brightness-weighted circular mean of hue.
  Modes: reactive, spot, spatial (<= 8 strip writes per tick, change
  threshold 6), scene, effect; per-saver scene defaults.
- POSTs go through the main process (the relay sends no CORS headers), 2 s
  timeout, no transport retry; the renderer backs off 5 s doubling to 5 min.
  GET is allowed only to snapshot and restore the room; a GET whose body does
  not parse is a failure, because restoring from nothing would leave the
  room lit.
- **Off with no URL by default** -- "the URL is the entire capability". On
  release fixtures keep their last colour; blackout was rejected because the
  room may be occupied.

## Consequences

Readback cost must be measured on a discrete GPU, never only on Apple
Silicon.

## Porting to egui

**Keep** the 1 Hz cap, enabled-only registration, colour maths, modes,
backoff and defaults. CORS and the main-process hop disappear: any thread
can POST. Since the Rust savers already render into their own RGBA8 target
before the blit, a GPU downscale plus an async PBO read becomes possible.
Not started.
