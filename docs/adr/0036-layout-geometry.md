# 0036. Layout geometry: contain-fit, a logo gap between the halves, optional side borders, dual by default

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2025-12-30 (24da509 gap and borders); 2026-01-16 (contain, PR #4; bottom logo removed, e6d918b); 2026-10-03 (freeze geometry, PR #332) |
| **Area** | UI |
| **Evidence** | `styles.css:144, 195, 415-490`; `renderer.js` `setCenterGap` :2163, `setBorderWidth` :2175, init :4970-4981; `src/main/index.js:128` |
| **egui verdict** | Keep |
| **egui status** | Done |

## Context

- Single view was briefly `object-fit: cover`; it was reverted the same day
  to `contain`, "ensures the entire video source is displayed without
  cropping".
- A bottom logo for single view existed for two weeks; since 2026-01-16 its
  markup is always hidden (dead).
- Why the centre gap exists is not recorded; plausibly the seam between two
  halves of the wall.

## Decision

- Always show the whole source, letterboxed, never stretched. Black-bar
  cropping sits on top of this ([0015](0015-crop-black-bars-by-snapping-to-shapes.md)).
- **Dual:** a black centre column `centerGap` px wide (default 60, slider
  20-200) with the Schuberg Philis logo contained in 90% x 80% of it.
- **Side borders:** black bars `borderWidth` px (default 0, slider 0-200)
  outside the video area, in both views.
- **Single:** no divider, no logo.
- Freeze, the pickers' gap column and the layout diagram must use the same
  gap -- #332 is what happens when they drift (a solid black freeze).
- **Default view:** the renderer meant `layoutMode: null` to mean "detect
  from the screen" (aspect >= 3 gives dual), but main merges its own default
  `'dual'` over it, so a fresh install is always dual and the detection
  never runs. README and USER_GUIDE describe the detection as live.

## Porting to egui

**Keep.** Prototype: done -- `dual_rects`, `wall_rect` and `logo_rect` are
pure functions with tests, used by the wall, the HUD and the Settings
diagram alike, so they cannot drift. Contain-fit in `paint_wall`.
`layoutMode: null` reads as dual, matching what Electron actually does;
decide whether real screen detection is wanted before adding it.
