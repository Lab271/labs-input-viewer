# 0003. Model the wall as two halves, never N panels

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2025-12-30 (77acd52); reaffirmed 2026-08-20 (#261, #244, #245 closed not planned) |
| **Area** | Product |
| **Evidence** | `renderer.js` `setLayout` :2104, `state.layoutMode` :106; #261, #244, #245; #243 |
| **egui verdict** | Keep |
| **egui status** | Done |

## Context

The wall is a 6000x1200 strip. From the first commit the app showed either
one picture across it (single) or two side by side (dual). In 2026-08 three
proposals tried to generalise that: an N-feed refactor (#261), per-panel
layouts (#244) and picture-in-picture (#245). All three were closed as not
planned on 2026-08-20: "the wall is not going to be used this way".

Cost was part of it. Two capture streams already cost 1.83 cores in
Chromium's `VideoCaptureService` (#243), and four had never been measured on
the wall host.

## Decision

- Two slots, `left` and `right`.
- **Dual** shows both. **Single** shows the left slot across the whole wall.
- The same device may be on both slots.
- Anything N-shaped -- dropdown thumbnails for other inputs -- is done
  without holding more streams ([0032](0032-input-thumbnails-are-snapshots.md)).

## Consequences

- In single view with Multi-view on, the right slot keeps its own input,
  which reappears on `D` ([0031](0031-what-a-picker-tap-sets.md)).
- Starting in single view once left the right slot with no stream, so `D`
  showed a black half on the wall; `ensureRightStream()` (PR #324,
  2026-10-02) fixes that.

## Porting to egui

**Keep.** The prototype holds `sides: [Option<String>; 2]` and paints one or
two halves (`app.rs`). Electron's per-side field duplication (10 fields, 15
ternaries per #261) is an implementation shape, not the decision: an array
of two keeps the constraint without the duplication. `ensureRightStream`'s
lesson is honoured by opening streams for whatever the visible sides show,
every frame (`sync_streams`).
