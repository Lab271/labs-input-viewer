# Open questions, defects and drift

Found while reconstructing the ADRs (2026-10-12, against `main` at 787375b,
v3.4.1). None of these is a decision; each needs an owner. Line numbers are
from that commit.

## Decisions to make before porting

D1 and D3-D6 decided 2026-10-13; see the ADRs' "Decided" sections.

| # | Question | ADR |
|---|---|---|
| D1 | Is reference-based no-signal detection still required? **The wall runs with no references**, and when they were removed is not recorded. | [0014](0014-detect-no-signal-against-references.md) |
| D2 | How is the screensaver dismissed? USER_GUIDE says any mouse movement, key or touch; the code accepts only a mouse shake, `V`, or signal returning. | [0023](0023-start-after-5-min-rotate-every-10.md) |
| D3 | An update prompt or error dialog can sit modal over an unattended wall. What should an unattended update look like? | [0048](0048-package-and-update-from-the-public-repo.md) |
| D4 | Where does the wall install the app? `startup.ps1` launches `C:\Program Files\Input Viewer\Input Viewer.exe`, NSIS is `perMachine: false` (default `%LocalAppData%\Programs`), Velopack installs per user. | [0006](0006-app-and-wall-setup-live-in-separate-repos.md) |
| D5 | Single-instance lock, display-sleep inhibitor, crash relaunch: app, startup script, or neither? | [0005](0005-no-single-instance-sleep-inhibit-or-relaunch.md) |
| D6 | Sign the builds? | [0049](0049-builds-are-unsigned.md) |
| D7 | Should the egui app's screensaver seeds reproduce Electron's (port mulberry32 + FNV-1a)? | [0022](0022-seed-every-activation.md) |
| D8 | Picker size on the wall: 128x72 tiles fill only the top-left of each ~3000 px half. | [0033](0033-controls-laid-over-the-wall.md) |
| D9 | `layoutMode: null` -- real screen-based detection, or drop the idea and default to dual (what actually happens)? | [0036](0036-layout-geometry.md) |
| D10 | Number keys silently override per-half choices in dual + Multi-view. Keep, or a per-side key scheme? | [0031](0031-what-a-picker-tap-sets.md) |
| D11 | Should screensavers get an app-state parameter? Weather (module singleton) and Split Flap (`setMessages`) each worked around its absence differently. | [0021](0021-screensaver-registry-contract.md) |

## Defects in the Electron app

Filed as issues on 2026-10-12. E16 was already tracked in #340.

| # | Defect | Where | Issue |
|---|---|---|---|
| E1 | The low-resolution retry requests audio with the **video** device id -- the bug #151 fixed on the main path. | `renderer.js:927-929` | #347 |
| E2 | The reference probe saves nothing: the full scan runs on every cycle anyway, for a diagnostic trace, even with no sink attached. | `detection-simple.js:436-442` (#163, 332fcc3) | #348 |
| E3 | `checkNoSignal()` passes the reference *list* to `compareFrames` and would always return false. Unused. | `detection-simple.js:241` | #349 |
| E4 | `force_release` without a bump builds artifacts and never publishes them. | `release.yml` (publish needs version-bump) | #350 |
| E5 | `chore(deps*)` commits never bump, so Electron/Chromium security updates never release without a manual `bump`. | `release.yml:53-91` | #351 |
| E6 | `settings.json` writes are not atomic; a corrupt file loads as defaults and is overwritten by the next save. | `src/main/index.js` `saveSettings` | #352 |
| E7 | Main and renderer keep different default settings (`layoutMode`, `centerGap`, `borderWidth`, volumes, remote keys, `cropLetterbox`, `aspectHint`). | `index.js:124-150`, `renderer.js:366-405` | #353 |
| E8 | The update error dialog says "Failed to download update" for a failed **check** too. | `index.js:850-860` | #354 |
| E9 | #86's proposed guard -- check every `url`/`path` in `latest*.yml` exists -- was never added. | `release.yml` publish job | #355 |
| E10 | Dead IPC: `get-settings-path` (caller removed in #334), `diag-log-reset` (never called). | `src/preload/index.js` | #356 |
| E11 | Dead markup: `#bottom-logo` (always hidden since 2026-01-16), `.input-label`, the `renderDropdownVolumeControls` shim. | `index.html:36`, `styles.css` | #357 |
| E12 | The same device on both halves opens two audio tracks; the audio may play twice. | `renderer.js` `startVideoStream` | #358 |
| E13 | The WebCodecs read copies the whole frame (~33 MB at 4K) every 1.6 s, contradicting #61's "copies only the region". Unmeasured on the wall. | `frame-source.js` | #359 |
| E14 | shadercheck is not run in CI; a shader regression merges unless someone runs it. | `ci.yml`, `release.yml` | #360 |
| E15 | macOS auto-update has never been confirmed end to end on the unsigned build. | #86 checklist | #361 |
| E16 | A 2026-10-05 over-crop of a mirrored MacBook on the wall is undiagnosed. | #316, #340 | #340 |

## Documentation drift

| Says | Actually | Where |
|---|---|---|
| Electron 43.x | 44.4.5 | `.claude/CLAUDE.md`, `index.js:113` |
| health thumbnail 32x18 | 384x216 since #314 | CLAUDE.md, `stream-health.js:64` |
| presenter "behind HDBaseT" | 15 m HDMI since 2026-10-02 | CLAUDE.md |
| 12 savers, 30 baselines, "6 of the 12" | 29 savers, 29 baselines | CLAUDE.md, `shader-check.mjs:78`, `gl-base.js:206`, `post-fx.js:8` |
| input fades back in over 180 ms | snaps | `renderer.js:2066` |
| in-app compositor is rAF-driven | rVFC | #62 comment |
| `W` cycles weather states | `A` (W is wall mode) | `weather-states.js:7` |
| shortcuts have three consumers incl. the Settings table | the table was removed in #334 | `shortcuts.js` header |
| the legend slides | it fades since #334 | CLAUDE.md |
| `observeFrames` reads a 32x16 downscale every frame | 8x4 tiles at 1 Hz (#275) | `gl-base.js:652-664` |
| structure baselines: "GENERATED, do not hand-edit" | CLAUDE.md says hand-edit a single line, and practice follows it | `structure-baselines.js` header |
| layout is detected from the screen aspect | always dual (main's default wins) | README, USER_GUIDE, `settings.example.json` |
| "Settings shows the exact path" | it no longer does | `settings.example.json` |
| remote keyboard forwards "arrow key presses" | PgUp/PgDn primarily | Settings copy |
| screensaver ends on any mouse/key/touch | shake, `V` or signal only | USER_GUIDE |
| ~~log folder `%APPDATA%\input-viewer\logs`~~ | **resolved 2026-10-12:** it is `%APPDATA%\input-viewer\` on the wall, as labs-videowall-config says | -- |
| branch types | CLAUDE.md omits `test/`, DEVELOPMENT.md has it | `.claude/` |

## Not recorded anywhere

- Why the capture request says 4096 wide rather than 3840.
- Why OpenCV.js was dropped for simple frame comparison (the 11 MB spike is
  still in `spikes/`).
- Why the DVD logo is the fallback beyond being first; why 10 minutes for
  rotation.
- Why there is a separate VERSION file; why vitest is not a devDependency.
- When and why required status checks disappeared from `main`'s protection.
- How the wall actually receives updates.
