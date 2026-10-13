# 0030. Drive the room's lights from the screensaver: sparse readback, 1 Hz, only when enabled

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-11 (41513aa, PR #199, #59); 2026-08-21 (49707c5, PR #275); GET snapshot 2026-08-25 (PR #293) |
| **Area** | Screensavers |
| **Evidence** | `gl-base.js:165-302, 400-457, 603-622`; `artnet-sync.js:1-150, 300-345, 440-470, 783-787`; `src/main/index.js:442-504` |
| **egui verdict** | Keep |
| **egui status** | Done |

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

Done, in `input_viewer_egui/src/artnet/`, with the seven Electron test files
ported case for case (the two that the port makes true by construction --
no overlapping requests, no pending timers -- are noted, not ported):

- **Client** (`client.rs`): artnet-sync.js as a synchronous state machine
  over a `Transport` trait, owned by one worker thread (`worker.rs`, ureq,
  2 s timeout, no retry, http(s) only, an unparseable GET is a failure). The
  UI thread only sends it messages; nothing on the render path waits on the
  LAN. The saver lifecycle reaches it from one place in `App::logic`
  (activate / rotate / release by comparing the running saver), per
  [0052](0052-run-background-work-in-app-logic.md).
- **Readback** (`observer.rs`): one NEAREST blit of the saver's output down
  to 64x32, one `readPixels` into a PBO, a fence, collected on a later
  frame. Measured on an M3 Pro (GL 4.1 over Metal, 2560x1440): 0.1-0.3 ms
  of render-thread time per read, collected 2-30 ms after issue, once a
  second. 32 tile-sized blits, the literal port of the 8x4 grid, cost
  9-20 ms per read there, so each 8x8 tile now samples its whole grid cell
  on an 8x8 lattice instead of an 8x8 block at its centre (same layout and
  maths, wider coverage, still point samples). With Art-Net off the cost
  is one atomic load per frame. **Still to measure on the wall's discrete
  GPU**, per the consequence above.
- Painter savers (Starfield, DVD logo, Lissajous) feed the observer through
  a paint callback that samples egui's framebuffer, since in Electron every
  saver went through gl-base. The split-flap board does not: in Electron it
  was a GL runtime too and fed frames even with no saver up.
- Settings: the `artnet*` keys are typed fields with main/index.js's
  defaults (maxBrightness 0.8: Electron's client alone fell back to 1, see
  E7), read leniently so a mistyped key cannot get the file moved aside.
- A target change releases and then re-activates the running saver, where
  Electron only released. `--artnet-url=URL` turns Art-Net on for a mock or
  review run, which never writes settings.
