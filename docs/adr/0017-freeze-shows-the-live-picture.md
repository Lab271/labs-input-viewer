# 0017. Freeze shows exactly the picture that was live

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-01 (PR #314, it used to stretch); 2026-10-03 (PR #332, centre gap) |
| **Area** | Signal |
| **Evidence** | `renderer.js:538-610` (`toggleFreeze`, `drawFitted`, `captureFrame`) |
| **egui verdict** | Adapt |
| **egui status** | Done |

## Context

Freeze (Space) holds the picture -- for a presenter changing slides
privately, for example. It snapshotted each feed into a canvas, and twice
that snapshot differed from the live view: it stretched the frame over the
whole area (fixed in #314), and it read a nonexistent `layoutGap` and drew
solid black (fixed in #332).

## Decision

Snapshot both visible feeds with the same crop and contain-fit as the live
view, keeping the dual-view gap. Hide the live videos while frozen. Detection
pauses; the health monitor keeps running. Show a green "LIVE" for 1 s on
unfreeze.

## Consequences

Freeze needs its own copy of the layout geometry, which is how it drifted
twice.

## Porting to egui

**Adapt:** natively, freeze is "stop uploading new textures". The picture
cannot differ from the live one because it **is** the live one, drawn by the
same code. Prototype: done (`update_frames` skips the upload while frozen,
a FROZEN badge shows). Missing: the 1 s LIVE flash on unfreeze.
