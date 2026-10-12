# 0032. Input thumbnails are snapshots, swept one stream at a time

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-20 (#242 decision, PR #265, 10b6fd2) |
| **Area** | UI |
| **Evidence** | `input-thumbnails.js` (`FRAME_TIMEOUT_MS = 2500`, 256x144 JPEG q0.72); `styles.css:34`; #242, #261 |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

Operators wanted to see what each input carries before switching. Live
previews would have meant holding N streams -- the #261 refactor, closed
unbuilt ([0003](0003-model-the-wall-as-two-halves.md)). The first layout used
full-width tiles and measured 292x164 each, 1166 px of content against a
640 px cap.

## Decision

- An input already on screen is snapshotted from its live picture: no
  stream, no negotiation.
- Any other input gets a temporary capture, released the moment a frame is
  drawn. **At most one extra stream at a time**; the sweep is sequential and
  rows fill in as stills land. A second sweep while one runs is ignored.
- A device that never presents a frame times out at 2500 ms and stays a
  placeholder -- as does an exclusive-access device. **A failed capture is a
  normal outcome, not an error.**
- Stills are cached by device id, so re-opening shows the last one
  immediately while a fresh sweep replaces it. Staleness is accepted: no
  refresh timer.
- Tiles are a fixed 128x72 with the name underneath; the strip scrolls
  sideways. Thumbnails use cover (the tile is not the wall).
- Mock inputs take their own branch, or the feature would be untestable in
  test mode.

## Consequences

2500 ms is a guess, never verified on real cards.

## Porting to egui

**Keep.** Prototype: 128x72 tiles from a cache of the last frame each device
showed on screen (`stills`), so on-screen inputs are right and others show
"no preview" until they have been seen. Missing: the one-at-a-time sweep of
off-screen inputs -- natively, open the device, take one frame, drop it,
which fits the slot model.
