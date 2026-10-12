# 0024. Scale savers by area and by room light, not by axis

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-03 (b6fb632, #87); 2026-08-04 (bc88fec #106, e0c2e31 #110, #88); supersample caps 2026-08-07 (#116, PR #134) |
| **Area** | Screensavers |
| **Evidence** | `gl-base.js:80-104` (`resolveSampleCount`), `:932-1086`; `plasma.js:9-24, 182-198`; #88 |
| **egui verdict** | Keep |
| **egui status** | Not started |

## Context

#88: "particle screensavers are near-invisible on the 6000x1200 videowall".
The wall is 10 m x 2 m: one pixel is 1.67 mm, and a 1-2 px dot is about 1.3
arcmin at 8 m. Its area is 3.47x 1080p but its short axis barely more. The
room is lit, which lifts the black floor; 12% washout is the sizing case.

## Decision

- **Point size:** scale by sqrt(area / 1080p), clamped 1-4x, minimum 4 px
  above 1.5x 1080p area.
- **Particle count:** texture side scaled by sqrt(area ratio), multiples of
  8, capped, never scaled down.
- **Brightness:** x1.6 only above 3x 1080p ("big room"), so desk monitors
  are unchanged.
- **Supersampling goes the other way:** at most 2 samples above 3x HD and 4
  above 1.4x HD -- at the wall, 4 samples would cost ~14x a single-sampled
  1080p frame.
- **Art direction for a 5:1 lit wall** (stated in Plasma, echoed in others):
  put fine detail in lightness and vary hue only at low frequency; dim whole
  stretches with a slow low-frequency energy mask so the eye can rest; keep
  a floor ("a quiet region should be dim plasma, not an absence of image" --
  a black patch reads as a dead panel); add highlights after the mask so
  they exceed 1.0 for bloom.

## Consequences

Thresholds are hard-coded against 1920x1080. The art rules live in shader
comments, not in a written policy.

## Porting to egui

**Keep.** The scaling functions depend only on pixel dimensions: port them
verbatim as pure functions with tests. The art rules travel inside the GLSL.
Not started -- Plasma is a pure fragment shader and needs none of them.
