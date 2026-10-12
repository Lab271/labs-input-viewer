# 0028. Verify shaders with a direct harness and structure baselines; measure speed only on a real GPU

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-06 (shadercheck, PR #132); baselines 2026-08-09 (#156) to 2026-08-18 (#227); #155 (backtick check); #225 (open) |
| **Area** | Screensavers |
| **Evidence** | `scripts/shader-check.mjs:9-80`; `screensavers/shadercheck.js:1-140`; `structure-baselines.js`; `scripts/update-structure-baselines.mjs`; `test/screensaver-seed.test.js:273-345`; `scripts/check-glsl-templates.mjs`; `raymarch.js:39-64, 113-131, 206-208` |
| **egui verdict** | Adapt |
| **egui status** | Not started |

## Context

- `npm test` has no WebGL2 context, and headless-gl would add a native build
  to `npm ci`.
- The first shadercheck went through the registry and reported **60/60
  passing while the driver rejected a shader** -- the fallback
  ([0021](0021-screensaver-registry-contract.md)) hid it.
- Raymarch compiled cleanly and rendered wrongly (#140); an unguarded
  `normalize()` gave NaN only on a real GPU.
- Physarum rendered a flat gradient that passed every pixel check.
- A backtick inside a GLSL comment ends the JS template literal; it "broke
  the build four times in one session" (#155).
- Several PRs claimed frame cost "cannot be measured headlessly" because
  they had seen SwiftShader numbers. Headless Chrome gives ANGLE/Metal on an
  M3 Pro, and every figure in #225 was measured that way.

## Decision

- **shadercheck** forces SwiftShader (a deterministic yes/no beats a fast
  one), calls `create()`/`start()` **directly**, runs each saver with a
  pinned seed twice and a fresh one, and checks pixels: no NaN, <= 0.1%
  negative, <= 90% blown out, at least one lit pixel. If you touch it,
  re-verify by injecting a type error and confirming a non-zero exit.
- **Structure baselines:** per-saver edge density, minimum over seeds of the
  maximum over frames 5 and 12, failing on a 35% drop (and an absolute 0.0015).
  `npm run baselines` regenerates all 29 lines from one noisy measurement, so
  a deliberately redesigned saver's line is edited by hand instead; entries
  stay in descending order (tested).
- **Static checks in `npm test`:** GLSL extractable, balanced braces,
  malformed literals, `iSeed` declared, used and validly swizzled; the user
  guide's saver list matches the registry.
- **Speed is measured on a real GPU at 6000x1200.** Raymarch's loop budgets
  lowered one at a time gained nothing (16.3 to 16.4 fps); lowered together,
  16.3 to 31.3 fps (register occupancy). Budgets change as a set.

## Consequences

- **shadercheck is not in CI**; a shader regression merges unless someone
  runs it locally.
- No saver has been measured on the wall host (#225 open; the measuring
  script `shot.mjs` is not committed). The Art-Net readback cost was measured
  at 4% on Apple Silicon and was 1.4 vs 118.9 fps on the wall's discrete GPU
  ([0030](0030-art-net-reactive-lighting.md)).
- Low baselines (Plasma 0.0001, Wave Tank 0.0007, Julia 0.0001) and Tree
  Growth (0) are unprotected by the drop rule.

## Porting to egui

**Adapt.** Compile every extracted shader in `cargo test` -- a hidden 1x1
glow context, or naga/glslang validation -- which closes the not-in-CI gap.
Keep the direct-call rule and the pixel checks; the prototype's
`INPUT_VIEWER_SCREENSHOT` hook (`review.rs`) already renders headlessly.
Re-measure baselines and frame cost in the Rust build on the wall's NVIDIA
driver: budgets tuned against ANGLE/Metal may not hold under desktop GL.
Retire the backtick check once shaders live in `.frag` files.
