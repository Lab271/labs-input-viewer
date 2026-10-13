# 0014. Detect no-signal by comparing against operator-captured reference screenshots

| | |
|---|---|
| **Status** | Accepted -- but dormant on the wall |
| **Date** | 2026-01-11 (69fa6d5); several references 2026-08-09 (#161, PRs #160, #162, #166, #167); loop 2026-08-05 (#60, #61, PR #129) |
| **Area** | Signal |
| **Evidence** | `detection-simple.js:10-22, 31-43, 140-212, 406-616, 636-713`; `frame-source.js`; `renderer.js:4518-4845`; #83, #104, #150, #163 |
| **egui verdict** | Port, off the video path |
| **egui status** | Not started |

## Context

A capture card with nothing plugged in still sends frames: its own "no
signal" screen. The app learns what that looks like by having the operator
capture it. Approaches tried before: a vision model and template matching in
the Python v1 (d42a80e, 2d59fda), an OpenCV.js spike (a570933; why it was
dropped is not recorded). Motion detection was rejected (#159).

A card has several no-signal screens -- no cable, unsupported mode, HDCP
error -- so one reference was not enough (#161). Storing references at full
resolution made `settings.json` 21 MB for 12 references.

## Decision

- **Compare:** downscale the frame to <= 480x270 (nearest neighbour), compare
  every 4th pixel (32,400 samples). A sample matches if |dR|, |dG|, |dB| are
  each <= 30; the frame matches at >= 95% of samples. Early exit once a
  match is impossible (#104: 1.47 ms to 0.065 ms at 1080p).
- **Several references per device:** matching ANY one means no-signal.
  Stored as 480x270 PNG data URLs under
  `settings.noSignalReferences[deviceId].references[]` (28.4 MB to 3.11 MB
  for 20). Orphans (device id changed) are logged and offered for pruning,
  never auto-deleted.
- **Probe first:** 32 golden-ratio points, reject after > 5 misses -- tuned so
  it "must never reject what the full scan would match".
- **Debounce:** two agreeing results flip the state. No reference means "has
  signal".
- **Loop:** every 1600 ms wall clock, paced by `requestVideoFrameCallback`
  with a watchdog at 2x (rVFC never fires on a static feed). Frames via
  MediaStreamTrackProcessor, canvas fallback: "a working slow path beats a
  broken fast one".

## Consequences

- **The production wall runs with no references** (`.claude/CLAUDE.md`, PR
  #311). When and why they were removed is not recorded.
- The thresholds date from 2026-01 with no recorded tuning data.
- Two defects: the probe saves nothing because the full scan runs anyway for
  the diagnostic trace (`detection-simple.js:436-442`); and the legacy
  `checkNoSignal()` is dead and would always return false
  ([open questions](open-questions.md)).
- `VideoFrame.copyTo({rect})` crops rather than resizes; detection once
  compared a corner against the whole frame and never fired (#150, #163).

## Porting to egui

**Decide first:** is reference detection still a product requirement, or a
feature for hardware other than the wall? If kept, the algorithm and
thresholds are pure arithmetic and port as is; store references as PNG files
keyed by device id rather than base64 in JSON. The whole frame-source layer
(rVFC, WebCodecs, canvas fallback, the copyTo trap) is Chromium-specific
and disappears: the capture thread already has CPU frames. Keep the ~1.6 s
cadence that continues when frames stop.

## Decided 2026-10-13

D1: **port it**, with one constraint: it must never interfere with the video
stream. The comparison runs on the health thumbnail the app already takes
(no extra readback, no extra copy of the frame), off the paint path, and
never holds up a frame.
