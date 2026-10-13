# 0048. Package per platform and auto-update from the public source repo, asking first

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2025-12-30 (b639ad1, prompt-first); 2026-04-17 (7aa05e8, feed moved to the source repo); 2026-06-08 (9aebf8f, zip for updates); 2026-08-04 (#86, artifactName) |
| **Area** | Delivery |
| **Evidence** | `input_viewer_electron/package.json` `build`, `publish`; `src/main/index.js:58-80, 313-325, 797-889`; `release.yml` publish job; #86, #105, #249 |
| **egui verdict** | Keep; silent updates |
| **egui status** | Done (silent) |

## Context

- A separate `input-viewer-releases` repo existed so updates could come from
  a public repo while the source was private (inferred). Its metadata was
  never uploaded, so updates were broken; it was dropped once the source was
  public.
- **#86:** `productName` "Input Viewer" has a space. electron-builder named
  files `Input Viewer-...`, GitHub's upload turned that into
  `Input.Viewer-...`, and `latest-mac.yml` said `Input-Viewer-...`. **Every
  macOS update 404'd for three releases.** The guard #86 proposed -- check
  every `url`/`path` in the metadata exists -- **was never added** (the
  publish job uploads the yml files unchecked).

## Decision

- **macOS:** universal DMG + ZIP (the ZIP is what the updater applies), with
  an explicit `artifactName`.
- **Windows:** x64 NSIS, `oneClick: false`, `perMachine: false`, install
  directory changeable, **`deleteAppDataOnUninstall: false`** (settings
  survive a reinstall), explicit `artifactName`.
- **Updates:** electron-updater from GitHub Releases on
  `LAB271/labs-input-viewer`, never in dev. One check, 3 s after the window
  is ready. "Download / Later", then "Restart now / Later"; installs on quit
  if postponed. Download progress shows as a toast.

## Consequences

- **On an unattended wall**, the prompt -- or the error dialog, which says
  "Failed to download update" even when only the check failed, e.g. offline
  at boot -- sits over 6000x1200 until someone clicks it.
- macOS clients on 2.6.1-2.6.2 were stranded by #86.
- How the wall actually receives updates is documented nowhere.

## Porting to egui

**Keep** the public-repo feed and ask-first flow; **Adapt** the tooling.
Prototype (`release-egui.yml`, `update.rs`): Velopack -- a per-user
`Setup.exe` (no admin) and a universal macOS `.pkg`, delta updates, the same
two prompts as an in-app dialog, install on quit. It shares the Releases page
with Electron safely: tags `egui-v*`, **never marked latest** (Electron's
updater reads `/releases/latest`), and a custom update source that finds the
newest `egui-v*` release itself, because Velopack's GitHub source only reads
the ten newest releases. Both platforms upload into one release, and a step
fails the run if they ever produce the same file name -- the class of bug
#86 was.

**Decide:** an unattended wall needs a non-modal update path -- check
silently, show a small "update ready" note, install on the next quit or a
nightly restart, and never show a modal for a failed check. And decide the
install location the wall's startup script will launch
([0006](0006-app-and-wall-setup-live-in-separate-repos.md)).

**Verified in CI** (#346): both installers build, and every file carries its
platform in the name (`InputViewerEgui-windows-Setup.exe`,
`InputViewerEgui-0.1.0-pr.346-macos-full.nupkg`, `releases.windows.json`...),
so the two platforms share one release without colliding. **Not yet
tested:** publishing, installing, and an update from one release to the next.

## Decided 2026-10-13

D3: **silent**. The app checks and downloads in the background, never shows
a dialog, and applies the update when it next restarts (the wall restarts it
on boot). Nothing may sit modal over the wall.

Built: `update.rs` checks 3 s after start and every 6 h, downloads without
asking, and applies on quit (`wait_exit_then_apply_updates`, no restart) or,
after a kill, at the next start before the window opens
(`get_update_pending_restart`). The only UI is a line in the Settings footer.
