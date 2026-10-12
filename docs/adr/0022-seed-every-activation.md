# 0022. Seed every activation from the clock, log the seed, and perturb within curated ranges

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-06 (PR #132, 4d60750) |
| **Area** | Screensavers |
| **Evidence** | `screensavers/seed.js:1-24, 74-76`; `gl-base.js:13-24, 44-52, 507-529`; `registry.js:218-227`; `plasma.js:64-73`; `test/screensaver-seed.test.js:309-370` |
| **egui verdict** | Adapt |
| **egui status** | Partial |

## Context

`gl-base.js` resets `iTime` and `iFrame` to 0 on every start. PR #132 found
**7 of the then 12 savers replayed bit for bit**; Strange Attractor used
`Math.random()` but converged on the same attractor within a second anyway.

Free randomness is no better: "most of these parameter spaces are
degenerate". PR #132's first Gray-Scott regimes were wrong for 4 of 8.

## Decision

- Seed: FNV-1a over `String(Date.now())` (hashed because a timestamp's high
  bits barely change), into mulberry32. Logged by the registry on every
  activation; `?seed=` in the preview reproduces it.
- Shader savers get `uniform vec4 iSeed` -- four uncorrelated values, fixed
  per activation. JS savers get `createRng(seed)` with
  `range/around/int/pick/chance/sign/phase`, built in `create()` so choices
  survive a stop/start.
- **Perturb, don't randomise.** Seed only what changes the look
  meaningfully, from curated lists or tight bands (Plasma's `baseHue` table,
  the Gray-Scott regime table with a test). Comment every bound that carries
  weight.

## Consequences

Any look on the wall can be reproduced from its log line. The constants are
tuning; re-deriving them in Rust would lose it.

## Porting to egui

**Adapt.** `iSeed` and the curated GLSL tables carry over unchanged with the
shader source. JS-side ranges must be ported by hand, verbatim.

Prototype: a seed per activation from the clock, logged and shown for 4 s on
screen, `iSeed` passed to Plasma. **But it uses xorshift64\*, not mulberry32 +
FNV-1a**, so an Electron seed does not reproduce in the egui app. Port
mulberry32 if seeds should be shareable between the two -- they should,
while both run.
