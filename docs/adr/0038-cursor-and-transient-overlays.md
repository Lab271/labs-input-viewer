# 0038. Hide the cursor; confirm switches with a toast only when nothing else names the input

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2025-12-30 (cursor, 3b5bd3d); 2026-10-03 (cursor kept over controls, toast rules, PR #334) |
| **Area** | UI |
| **Evidence** | `styles.css` `body { cursor: none }`; `renderer.js` `showCursor` :3880, `showInputName` :2250, freeze badge :538 |
| **egui verdict** | Keep |
| **egui status** | Done |

## Decision

- **Cursor:** hidden; shown for 3 s after movement; never hidden while the
  controls or Settings are open, or over the legend.
- **Input-name toast:** 2 s, 80 px from the bottom, 24 px text, one restarted
  timer (not one per call); suppressed while the pickers are open, since
  they already name what is on each half.
- **Freeze badge:** red FROZEN, then a green LIVE for 1 s on unfreeze.
- Sizes are chosen for reading at a distance.

## Porting to egui

**Keep.** Done: toast (2 s, suppressed while the pickers are open), red
FROZEN and a green LIVE for 1 s on unfreeze, and the cursor hidden
(`CursorIcon::None`) 3 s after the pointer last moved, never while the
dropdown, the legend or Settings is open.
