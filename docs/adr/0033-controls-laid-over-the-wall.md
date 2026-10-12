# 0033. Controls are laid over the wall from the top edge, and close themselves

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-03 (dropdown 2b, PR #334); earlier panel 2026-01-23 (PR #23); touch 2026-01-24 (PR #25) |
| **Area** | UI |
| **Evidence** | `renderer.js` `openDropdown` :3752, `CLOSE_BELOW_MARGIN_PX` :3785, `dropdownCloseLine` :3787, `DROPDOWN_IDLE_MS` :3808, `buildPicker` :2765-2880; `index.html` `#wall-pickers`; `test/settings-panel.test.js:341-374` |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

The wall is operated with a mouse, a touch screen, a clicker or a keyboard,
often from a distance, and is often left unattended. The earlier dropdown
was a 400 px panel with left and right columns. The 2b redesign (a Claude
Design handoff) removed the panel.

## Decision

- **No panel.** Opening fills the wall with one picker per half -- label,
  current input, a strip of tiles, that half's volume -- plus a capsule at
  the top: Dual/Single, output volume, Settings.
- **Opens** on hover over the top-edge tab (mouse), on tap (touch), with
  Enter/Space (keyboard).
- **Closes** when the pointer goes below the lowest picker row + 48 px --
  the lowest row is the volume slider, so reaching for it does not close
  them; on touch, a tap below that line. Also with Esc, and after **30 s with
  no pointer, touch or key activity**, so the controls never sit over an
  unattended wall. There is no close button (a 56 px Close pill was removed
  in review).
- Opening Settings closes the controls; they refuse to open while Settings
  is.
- The input-name toast is suppressed while they are open.

## Consequences

**Open design issue:** at 6000x1200 everything is the handoff's size and
fills only the top-left corner of each ~3000 px half. "It probably wants
scaling on the wall" (PR #334).

## Porting to egui

**Keep.** Prototype: open at the top edge, close below the area + 48 px, Esc,
30 s idle, toast suppressed, Settings button closes them. Missing: per-half
volume and output volume ([0010](0010-volume-belongs-to-a-side.md)), an
explicit touch tap-to-toggle and keyboard open, and the scaling question --
which egui makes cheap to answer (`pixels_per_point`, or a scale factor on
the area alone).
