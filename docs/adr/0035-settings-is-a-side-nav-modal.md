# 0035. Settings is a side-nav modal that autosaves and reports its own status

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-03 (PR #334); modal 2026-01-23 (PR #23); references UI 2026-08-09 (#161, PR #167) |
| **Area** | UI |
| **Evidence** | `settings-status.js`; `renderer.js` input table :3013, `renderLayoutDiagram` :2187; `index.html` Settings panes |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

#161 found that "a device with no reference is silently inert"; before
`settings-status.js`, a wrong remote-keyboard hostname or a dead Art-Net relay
was "only visible in the log file".

## Decision

- A 920x680 modal with four panes: **Inputs, Layout, Remote keyboard,
  Art-Net**. The shortcut table and the separate no-signal section were
  removed: the legend has the shortcuts, and capture moved into each input.
- **Autosave**, no Apply; sliders debounce at 300 ms.
- **Inputs:** Key (`1`-`4`, `·` past four, `–` when disabled), On, Name (an
  empty name restores the hardware label, shown as the placeholder), Startup
  (a radio that can be cleared), No-signal (capture from the half the
  device is on; "Put this input on the wall first" when it is not). A
  disabled input still on screen says "Still on the wall until you switch".
- **Nav dots** report state: inputs with no reference, integrations On/Off,
  orange when on but broken.
- **Layout** has a to-scale drawing of the wall.
- Names are rendered with `textContent`, never `innerHTML` (an XSS test).
- Pure status logic lives in `settings-status.js`.

## Porting to egui

**Keep.** Prototype (`app/settings_ui.rs`, `settings_status.rs`): Inputs and
Layout panes; autosave; the Key column rules; names trimmed on leaving the
field; a start-up choice with "the inputs last on screen" as the cleared
state; a live status line and dot per input; the wall diagram to scale,
drawn by the same geometry code as the wall; the wall itself follows the
sliders live. Missing: the No-signal column
([0014](0014-detect-no-signal-against-references.md)), Remote keyboard and
Art-Net panes, and "still on the wall until you switch" (in the prototype a
disabled input on screen goes to no-signal immediately). The `innerHTML`
concern does not exist in egui.
