# 0039. Visual language: SBP colours, system fonts only, sized for a lit room at a distance

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-03 (PR #334; font decision recorded the same day); chip floor 2026-08-19 (PR #260) |
| **Area** | UI |
| **Evidence** | `styles.css:5-35, 599`; project memory "No brand font in the public repo" |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

The design handoff used TT Interphases, which is licensed to Schuberg Philis
for internal use ("do not redistribute outside SBP"). This repo and its
installers are public. A `local()` "use it if installed" stack was offered
and declined in favour of plain system fonts.

## Decision

- Colour tokens: `--sbp-blue #1E80ED`, turquoise, signal green and orange;
  container radius 16/16/16/2 px ("a guess"), pills, 200 ms fades.
- **System fonts only. Never ship TT Interphases** in any form.
- 12 px minimum text; 44 px minimum hit targets.
- Icons are placeholders until the design file's are exported.

## Porting to egui

**Keep.** egui embeds fonts in the binary by default -- easy to get wrong
here. Use egui's bundled fonts (Ubuntu-Light and Hack, both redistributable),
which the prototype does, or a system font loaded at runtime; never embed
TT Interphases. The prototype forces egui's dark theme (a light system theme
put white panels over the wall). Re-derive the 12 px floor and 44 px targets
for egui points at the wall's `pixels_per_point`, and map the colour tokens
into an egui `Style`.
