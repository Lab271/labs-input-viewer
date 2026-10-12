# 0027. Keep network access outside the savers; weather is opt-in and coarsened

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-11 (c16bc06, PR #198, #101) |
| **Area** | Screensavers |
| **Evidence** | `weather-source.js:1-99`; `weather.js:45-60, 83-86`; `weather-states.js`; `src/main/index.js:142-147` |
| **egui verdict** | Keep |
| **egui status** | Not started |

## Context

The registry's `try { create(); start() }` cannot catch a fetch that rejects
after `start()` returns. A timer owned by a saver would leak through `stop()`
into every later saver. And the request sends the wall's location to a
third party.

## Decision

- A module-level poller (`installWeatherSource`) independent of the saver
  lifecycle; the saver only reads `currentReading()`.
- Open-Meteo: no key, no account, CC-BY. Every 15 min; backoff 1 min to 1 h;
  10 s timeout; stale after 30 min; expire after 12 h. Coordinates rounded to
  2 decimals before they leave the machine.
- **Off by default** (`weatherEnabled: false`) -- "the only feature here that
  reaches a third party". Default location 52.37 / 4.89.
- The scene eases between readings over 25 s. Canned states in
  `weather-states.js` exist for review only.

## Consequences

Works from the renderer only because Open-Meteo sends permissive CORS
headers.

## Porting to egui

**Keep** the split, the opt-in default, the rounding, the backoff and
staleness rules. The poller becomes a background thread (`ureq` is already a
dependency, for the updater). CORS stops mattering. The weather saver itself
is multi-pass with glyphs and needs a real port. Not started.
