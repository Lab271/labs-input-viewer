# 0029. Draw text in savers through a baked glyph atlas

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-08 (b2d5689) |
| **Area** | Screensavers |
| **Evidence** | `screensavers/glyph-atlas.js:6-19`; `artnet-sync.js:17-24` |
| **egui verdict** | Adapt |
| **egui status** | Partial |

## Context

`getContext('2d')` returns null on a canvas that has ever been WebGL2, and
rasterising and uploading a 6000x1200 frame of text every frame dominated
the cost.

## Decision

Bake glyphs once on a detached 2D canvas (browser `monospace` font), index
them in the shader, and share one atlas between ASCII Doughnut, Split Flap,
Matrix Rain and Weather.

## Consequences

The glyphs depend on the system's monospace font.

## Porting to egui

**Adapt.** The one-context-per-canvas problem does not exist in egui, but
bake-once is still the right performance choice. Rasterise with egui's own
font system (`epaint` already builds an atlas) or `ab_glyph`, and keep the
fonts redistributable ([0039](0039-visual-language.md)). The prototype draws
its painter-based savers' text with egui directly.

Weather's readout is done that way too: one line of square cells (the
atlas's layout, glyph at 0.82 of the cell) on the same dark plate, painted
with egui's monospace font over the GPU scene. It therefore skips bloom and
ACES, which in Electron only dimmed it slightly. Matrix Rain, ASCII Doughnut
and Split Flap, which index glyphs per pixel in the shader, still need a
real atlas.
