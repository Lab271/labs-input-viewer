# 0021. Screensavers are modules behind one synchronous contract, with a safe fallback

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-06-07 (aa71c72); seed argument 2026-08-06 (PR #132); `isAvailable` 2026-08-11 (PR #198, #101) |
| **Area** | Screensavers |
| **Evidence** | `screensavers/registry.js:3-20, 53-83, 107-136, 179-248, 265-277`; `weather.js:18-23, 45-60, 691` |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

29 savers draw on one shared canvas, one at a time. A shader that fails to
compile on unknown wall hardware must not leave the wall black. Weather is
the first saver that depends on something outside itself; an async start
would leave a blank canvas while a fetch failed.

## Decision

- Contract: `{ name, create(canvas, seed) -> { start, stop }, isAvailable? }`.
  `startScreensaver()` is **synchronous**.
- `create()` / `start()` are wrapped in try/catch. On failure, `stop()` the
  partly started instance to release its GL resources and start
  `SCREENSAVERS[0]`, the DVD logo.
- `isAvailable()` is optional and gates **only the random rotation**; a
  throw counts as false. Explicit selection (stepping keys, preview,
  shadercheck) ignores it, so such a saver must render something defensible
  with no data -- Weather's `FALLBACK` is a calm half-clouded day.
- Rejected on #101: an async `prepare()` (every caller becomes async, an HTTP
  round trip before the first no-signal), and a saver reporting its own
  failure mid-frame (re-entrancy, and still blank while it fails).
- Selection by name ignores case and whitespace; the active index is read
  from the registry, never cached by callers.

## Consequences

The fallback makes failures invisible to callers -- which is how the first
shadercheck reported 60/60 while the driver rejected a shader
([0028](0028-verify-shaders-and-measure-on-a-real-gpu.md)). Why the DVD logo
is the fallback, beyond being first and simplest, is not recorded.

## Porting to egui

**Keep** as a Rust trait: `fn create(seed) -> Result<Box<dyn Saver>>`,
`start/stop/paint`, optional `is_available()`. Prototype: four savers in a
`State` enum in `screensaver.rs`; Plasma falls back to the DVD logo if its
GL setup fails (`plasma::failed()`). `is_available` is ported (Weather only, `screensaver::pick_random`). Missing: a stepping
order that matches the Electron registry, and `stop()` releasing GL
resources (the prototype keeps compiled programs for the context's life,
deliberately, to avoid recompiling).
