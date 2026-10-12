# 0031. What a picker tap sets depends on view and Multi-view; key chips only where the key does the same

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-03 (`multiView`, PR #334); chips rule 2026-08-19 (PR #263) |
| **Area** | UI |
| **Evidence** | `renderer.js` `pickerPlan()` :2751, `setMultiView` :456; `test/settings-panel.test.js:222-250`; PR #263 |
| **egui verdict** | Keep |
| **egui status** | Done |

## Context

`multiView` (default on: "that is how the wall has always worked") lets the
two halves show different inputs. The number keys `1`-`4` call
`selectInput(side='both')`; a tap in the Left half picker sets one side. A
key chip next to a per-half picker documented "a key that does something
different from the control beside it" (#263).

## Decision

| View | Multi-view | Pickers | A tap sets | Key chips |
|---|---|---|---|---|
| dual | on | Left half, Right half | that half | none |
| dual | off | Both halves | both | 1-4 |
| single | on | Whole wall | left | 1-4 |
| single | off | Whole wall | both | 1-4 |

Turning Multi-view off brings the right half in line with the left; startup
enforces the same.

## Consequences

- `1`-`4` set both halves even with Multi-view on, silently discarding a
  per-half choice. Deliberate and documented; a per-side key scheme would be
  the alternative.
- The name collides with #244's "multi-view" (per-panel layouts, closed not
  planned, 2026-08-18); `multiView` only means "the halves may differ". It
  shipped in #334 without a record of the reversal.

## Porting to egui

**Keep** as a pure function with its table as the test. Prototype: done --
`picker_plan()` in `app.rs` with
`picker_plan_matches_the_electron_table`; `set_multi_view(false)` aligns the
right half.
