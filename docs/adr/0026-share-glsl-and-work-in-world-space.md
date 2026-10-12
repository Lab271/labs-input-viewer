# 0026. Share GLSL through one library, and work in world space divided by the short axis

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-07 (PR #138, #115, #114) |
| **Area** | Screensavers |
| **Evidence** | `glsl-lib.js:4-48, 51-86, 243-250, 262-318, 405-414` |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

- Clip space is stretched 5:1 on the wall: Boids' 0.06 separation radius
  became a 180x36 px ellipse (#114).
- Value noise shows lattice artefacts at wall scale.
- `fract(sin(dot(...)) * 43758.5453)` shows structure and correlates
  neighbouring samples (Boids neighbour sampling, White Particles respawns).
- Without a torus delta the flock tore at the screen edges.
- GLSL has no imports, so snippets were being copied.

## Decision

- `glsl-lib.js` holds snippets spliced into shaders as strings: integer
  bit-mixing hashes (Jarzynski & Olano, JCGT 2020) via `floatBitsToUint`,
  simplex 2D/3D, fbm with analytic derivatives, OKLab, `worldSpace`,
  `torusDelta` / `torusWrap`.
- World y runs -0.5..0.5 and x +-aspect/2; aspect returns only at
  `gl_Position`. A wider canvas shows **more**, not a magnified copy.
- New snippets are appended at the end of the file to avoid merge conflicts.

## Consequences

Snippets must be deduplicated when composed -- GLSL rejects a function
defined twice.

## Porting to egui

**Keep**, and the prototype already relies on it: `shaders/extract.mjs`
imports `glsl-lib.js`, substitutes `${GLSL.x}`, and writes a `.frag` that
runs on GLSL 3.30 unchanged (`floatBitsToUint`, `uint` and `gl_InstanceID`
are all core there).

Its limits, per saver:

| Kind | Savers | Port |
|---|---|---|
| Pure fragment shader | Plasma (done), Flow Field, Julia Family, Burning Ship, Moire, Bicycle Horizon | extract as is (+ supersample footer for two) |
| Shader with JS constants interpolated (`${SAMPLES}` etc.) | Boids, Aquarium, Pong, Starfield, Metaballs, ... | evaluate the template, not a regex |
| JS orchestration: ping-pong, instancing, CPU simulation | the other ~20 | rewrite the orchestration in Rust, keep the GLSL |
| Glyph atlas | Matrix Rain, ASCII Doughnut, Weather, Split Flap | Rust font rasteriser ([0029](0029-text-through-a-baked-glyph-atlas.md)) |

When the port owns the shaders, move them out of JS template literals into
`.frag` files -- which also retires the backtick hazard
([0028](0028-verify-shaders-and-measure-on-a-real-gpu.md)).
