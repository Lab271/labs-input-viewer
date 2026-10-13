# 0041. Settings are one JSON file in the app's data folder; network features default off; test mode never writes

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2025-12-30; mkdir fix 2026-01-13 (62eb8fb); mock guard 2026-08-19 (PR #259); example rewritten (#53, PR #129) |
| **Area** | Platform |
| **Evidence** | `src/main/index.js:87-88, 124-215`; `renderer.js:289-405` (`saveSettings` allowlist, `getDefaultSettings`); `settings.example.json`; README |
| **egui verdict** | Keep, and fix |
| **egui status** | Done |

## Context

Operators read and hand-edit `settings.json` on the wall. Inputs are keyed
by capture device id, because index order is not stable across reboots.

## Decision

- `<userData>/settings.json` -- `%APPDATA%\input-viewer\` on Windows
  (checked on the wall 2026-10-12; Electron uses package.json's `name`
  because there is no top-level `productName`).
- Main merges defaults shallowly over the file; on any read or parse error
  it logs and returns defaults. Writes are a plain `writeFileSync`.
- The renderer saves an explicit **allowlist** of keys, debounced 300 ms. "A
  key omitted from it is silently reset to its default on the next load."
- **Network features default off:** `weatherEnabled: false` (the only
  third-party call), `artnetEnabled: false` with no URL.
- **`--mock` never writes** -- one mock run would otherwise persist
  `mock-input-N` and could leave the wall starting on an input that will
  never exist again.
- No schema version, no migration code.

## Consequences

- **Not atomic.** A power cut mid-write leaves malformed JSON, which loads as
  defaults with only a console error; the next save then overwrites the
  file, and the wall's names and assignments are gone.
- **Two default sets** (main and renderer) that differ: `layoutMode` `'dual'`
  vs `null`, and main lacks `centerGap`, `borderWidth`, volumes and the
  remote-keyboard keys while the renderer lacks `cropLetterbox` and
  `aspectHint`.
- Hand edits made while the app runs are overwritten (README says so).
- The remote-keyboard API key is stored in plain text.

## Porting to egui

**Keep** the file, the key names, the defaults-off rule and never-write in
test mode. **Fix** the rest. Prototype (`settings.rs`), done:

- same camelCase keys; unknown keys kept in a flattened map and written back,
  so Art-Net, remote-keyboard and volume settings survive a round trip;
- one default set, in one place;
- atomic write (temp file + rename); a file that does not parse is moved to
  `settings.json.corrupt-<time>`, never overwritten;
- imports the Electron file once on first launch: layout and unknown keys
  carry over; **input names, enabled flags and the default input do not**,
  because Electron keys them by Chromium's salted deviceId hash and stores
  no label to match on;
- inputs keyed by AVFoundation `uniqueID` / Windows device path;
- mock **and** review runs neither read nor write.

Still open: a schema version field.
