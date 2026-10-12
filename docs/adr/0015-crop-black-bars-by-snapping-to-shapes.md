# 0015. Crop the card's black bars by snapping to known shapes

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-01 (PR #314); #321, #326 (2026-10-02); #329 (2026-10-03) |
| **Area** | Signal |
| **Evidence** | `content-box.js`; `renderer.js:1081-1127`; open: #316 (parked), #340 |
| **egui verdict** | Keep, and reconsider the source |
| **egui status** | Not started |

## Context

The Elgato 4K60 Pro MK.2 always delivers 3840x2160 (measured 2026-10-01:
`left stream: 3840x2160`). A source of another shape arrives fitted inside
it with pure black around it. A laptop at 3840x768 -- the wall's own 5:1 --
arrives as a band a third of the frame tall, and single view showed a 16:9
frame in the middle of a 5:1 wall.

A 96x54 thumbnail could not tell "dual half" (2.433:1) from 21:9 (2.370:1);
both measured 2.400. A mirrored Mac matched nothing until "MacBook" (1.542)
was added.

## Decision

- Measure on a 384x216 thumbnail every 2 s, shared with the health check
  ([0012](0012-judge-health-by-counting-frames.md)).
- A pixel is bar if luma <= 10; a row or column is picture if > 2% of it is
  above that. Bars must be symmetric within 4%.
- **Snap, never crop to the measurement.** The shape must be within 6% of
  one of `SHAPES` (5:1, 32:9, dual half, 21:9, 2:1, 16:9, 16:10, MacBook,
  3:2, 4:3, 5:4). One-axis bars get the exact centred crop; bars on both
  axes (a windowbox) get the shape's box, rounded down so picture is never
  cut.
- Change only after **3 identical results** (~6 s), 1% jitter tolerated.
  Undecidable samples are ignored. Reset on every new stream. An all-black
  or unmatched frame changes nothing -- which is what keeps a dark slide
  from being cropped.
- Applied as CSS `object-view-box`; `cropLetterbox: false` disables it.

## Consequences

- `drawImage` reads the full decoded frame whatever the crop shows, so
  detection keeps seeing the bars after cropping.
- **Known failures:** a false crop on dark content (#316, parked), and on
  2026-10-05 a mirrored MacBook cropped **into** the picture on the wall
  (undiagnosed; its `[Crop]` log lines are still needed).

## Porting to egui

**Keep** the maths (pure, unit tested) and the snapping rules. The crop
becomes UV coordinates on the textured quad -- a one-line change in the
paint call, and the freeze path gets it for free
([0017](0017-freeze-shows-the-live-picture.md)).

**Reconsider the source:** #340 proposes asking the Elgato driver for the
source resolution instead of inferring it from black bars. That is a KS
property query, hard from Electron and ordinary from Rust. Check first that
the query works while the app holds the card ([0006](0006-app-and-wall-setup-live-in-separate-repos.md)).
