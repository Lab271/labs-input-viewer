# 0025. An HDR post chain owns tonemapping; savers emit linear HDR; supersample in the shader

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-07 (post chain #112, #113; double-tonemap fix PR #140; supersampling #116, PR #134) |
| **Area** | Screensavers |
| **Evidence** | `post-fx.js:1-43, 122-200`; `gl-base.js:61-129, 697-720, 826-895`; `raymarch.js:17-37, 674-677`; `plasma.js:214-229` |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

- An 8-bit framebuffer gave no glow and clipped highlights; fading 8-bit
  trails by 0.04 alpha left permanent ghosts; a 6000 px bloom falloff banded.
- Only Raymarch encoded sRGB, so every other saver showed linear values as
  sRGB. Raymarch then got tonemapped and encoded **twice**, "lifting blacks
  by over 100%" (#140).
- WebGL's `antialias: true` is MSAA, which samples polygon edges; a
  fullscreen quad has none, so fractal filigree shimmered (#116).

## Decision

- Savers render linear HDR into an RGBA16F target. **Only the chain**
  tonemaps and encodes: soft-knee bright pass, 6-mip dual-filter bloom,
  composite with exposure, ACES (Narkowicz) or Reinhard-Jodie,
  `pow(1/2.2)`, and interleaved-gradient-noise dither of +-1/2 LSB.
- Bloom thresholds tuned per saver to ~70% of the **measured** peak (LDR
  savers peak at 0.36-0.80, accumulating ones up to ~119).
- Without float targets, a saver keeps an 8-bit direct path.
- Supersampling is generated in JS (`makeSupersampleFooter`): `main()` calls
  `mainImage` at 2, 4 or 8 rotated-grid offsets (8 is a rook pattern) and
  averages. Raymarch adds temporal jitter accumulated in the HDR target,
  turning itself off when frames take over a second.

## Consequences

Every saver shares one tonemap and one dither, so the wall's look is
consistent and tunable in one place (the preview's bloom keys).

## Porting to egui

**Keep.** Prototype: done for Plasma. `plasma.rs` runs the four post-FX
passes extracted verbatim from `post-fx.js`, into RGBA16F, with
`FRAMEBUFFER_SRGB` disabled so gamma is not applied twice (the #140 trap,
egui edition), and blits into egui's framebuffer.

Missing: the supersample footer is generated in JS, so `extract.mjs` does not
see it -- port `makeSupersampleFooter` to Rust before Julia Family and
Burning Ship (`antialias: 4`). Raymarch's accumulation loop needs its own
port. On desktop GL 3.3 float targets are core, so the 8-bit fallback is a
WebGL concern only.
