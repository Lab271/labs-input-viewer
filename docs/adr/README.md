# Architecture Decision Records

Input Viewer's design decisions, reconstructed from the Electron app's code,
issues and PRs in 2026-10, so it can be ported to Rust and egui
([0002](0002-port-to-rust-and-egui.md)) one conscious decision at a time.
How these are written and changed: [0001](0001-record-architecture-decisions.md);
the format: [0000-template.md](0000-template.md).

Each ADR ends with **Porting to egui**:

- **Verdict** -- *Keep*: port the behaviour as is. *Adapt*: same goal,
  different mechanism. *Drop*: an Electron or browser artefact. *Decide*: a
  product question to answer first.
- **egui status** -- where the prototype in
  [`input_viewer_egui/`](../../input_viewer_egui/README.md) stands: *Done*,
  *Partial*, *Not started*, *N/A*.

Defects, contradictions and doc drift found on the way are in
[open-questions.md](open-questions.md), including eleven product decisions
(D1-D11) worth settling before the port reaches them.

## Status board

### Product

| ADR | Decision | Status | egui verdict | egui status |
|---|---|---|---|---|
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions, and port by them | Accepted | Keep | N/A |
| [0002](0002-port-to-rust-and-egui.md) | Port Input Viewer to Rust and egui | Proposed | Decide | Prototype |
| [0003](0003-model-the-wall-as-two-halves.md) | Model the wall as two halves, never N panels | Accepted | Keep | Done |
| [0004](0004-one-fullscreen-window-wall-composition-is-external.md) | One borderless fullscreen window; the wall's composition is someone else's job | Accepted | Keep | Partial |
| [0005](0005-no-single-instance-sleep-inhibit-or-relaunch.md) | Single instance, display sleep and crash relaunch are not the app's job (by omission) | Open | Change: single instance and no sleep in the app | Done (lock, awake) |
| [0006](0006-app-and-wall-setup-live-in-separate-repos.md) | The app and the wall's setup live in separate repos; the startup script owns launch order | Accepted | Keep | Partial |

### Capture

| ADR | Decision | Status | egui verdict | egui status |
|---|---|---|---|---|
| [0007](0007-capture-at-native-resolution.md) | Capture at the card's native resolution, and log what was negotiated | Accepted | Adapt | Partial |
| [0008](0008-one-capture-session-per-device.md) | One capture session per device, shared by both halves | Accepted (Electron shape: one track per side) | Adapt | Done |
| [0009](0009-pair-audio-with-its-video-card.md) | Pair audio with its video card; missing audio never blocks video | Accepted | Adapt | Not started |
| [0010](0010-volume-belongs-to-a-side.md) | Volume belongs to a side; the OS volume is shown and set directly | Accepted | Adapt | Not started |
| [0011](0011-choose-startup-inputs-by-precedence.md) | Choose each half's input by a fixed precedence, and keep it across hot-plug | Accepted | Keep | Partial |

### Signal

| ADR | Decision | Status | egui verdict | egui status |
|---|---|---|---|---|
| [0012](0012-judge-health-by-counting-frames.md) | Judge capture health by counting frames, never by measuring motion | Accepted | Keep | Done |
| [0013](0013-reopen-by-releasing-the-device-with-two-backoffs.md) | Reopen a sick device by releasing it fully, on separate backoffs for faults and for dark | Accepted | Keep | Done |
| [0014](0014-detect-no-signal-against-references.md) | Detect no-signal by comparing against operator-captured reference screenshots | Accepted -- but dormant on the wall | Port, off the video path | Not started |
| [0015](0015-crop-black-bars-by-snapping-to-shapes.md) | Crop the card's black bars by snapping to known shapes | Accepted | Keep, and reconsider the source | Not started |
| [0016](0016-laptop-resolution-hint-off-by-default.md) | Keep the "set your laptop to 3840x768" hint off by default | Accepted (premise weakened since 2026-10-02) | Keep | Not started |
| [0017](0017-freeze-shows-the-live-picture.md) | Freeze shows exactly the picture that was live | Accepted | Adapt | Done |
| [0018](0018-composite-on-the-gpu.md) | Composite video on the GPU, with good downscaling | Accepted (Electron: CSS `<video>`; WebGPU path evaluated and not adopted) | Drop (decided by the platform) | Done |
| [0019](0019-one-no-signal-state-machine.md) | One no-signal state machine per side, with a downtime clock | Accepted | Keep | Partial |

### Screensavers

| ADR | Decision | Status | egui verdict | egui status |
|---|---|---|---|---|
| [0020](0020-split-flap-board-is-the-no-signal-display.md) | The split-flap board is the no-signal display, not a screensaver | Accepted | Keep | Partial |
| [0021](0021-screensaver-registry-contract.md) | Screensavers are modules behind one synchronous contract, with a safe fallback | Accepted | Keep | Partial |
| [0022](0022-seed-every-activation.md) | Seed every activation from the clock, log the seed, and perturb within curated ranges | Accepted | Adapt | Partial |
| [0023](0023-start-after-5-min-rotate-every-10.md) | Start a screensaver after 5 minutes dark, rotate every 10, never repeat the last | Accepted -- dismissal is Open | Keep; Decide on dismissal | Partial |
| [0024](0024-scale-by-area-and-room.md) | Scale savers by area and by room light, not by axis | Accepted | Keep | Not started |
| [0025](0025-hdr-post-chain-owns-tonemapping.md) | An HDR post chain owns tonemapping; savers emit linear HDR; supersample in the shader | Accepted | Keep | Partial |
| [0026](0026-share-glsl-and-work-in-world-space.md) | Share GLSL through one library, and work in world space divided by the short axis | Accepted | Keep | Partial |
| [0027](0027-network-access-outside-savers.md) | Keep network access outside the savers; weather is opt-in and coarsened | Accepted | Keep | Done |
| [0028](0028-verify-shaders-and-measure-on-a-real-gpu.md) | Verify shaders with a direct harness and structure baselines; measure speed only on a real GPU | Accepted | Adapt | Not started |
| [0029](0029-text-through-a-baked-glyph-atlas.md) | Draw text in savers through a baked glyph atlas | Accepted | Adapt | Partial |
| [0030](0030-art-net-reactive-lighting.md) | Drive the room's lights from the screensaver: sparse readback, 1 Hz, only when enabled | Accepted | Keep | Not started |

### UI

| ADR | Decision | Status | egui verdict | egui status |
|---|---|---|---|---|
| [0031](0031-what-a-picker-tap-sets.md) | What a picker tap sets depends on view and Multi-view; key chips only where the key does the same | Accepted | Keep | Done |
| [0032](0032-input-thumbnails-are-snapshots.md) | Input thumbnails are snapshots, swept one stream at a time | Accepted | Keep | Partial |
| [0033](0033-controls-laid-over-the-wall.md) | Controls are laid over the wall from the top edge, and close themselves | Accepted | Keep | Partial |
| [0034](0034-one-shortcut-list.md) | One shortcut list drives the keys, the legend and the docs | Accepted | Keep | Partial |
| [0035](0035-settings-is-a-side-nav-modal.md) | Settings is a side-nav modal that autosaves and reports its own status | Accepted | Keep | Partial |
| [0036](0036-layout-geometry.md) | Layout geometry: contain-fit, a logo gap between the halves, optional side borders, dual by default | Accepted | Keep | Done |
| [0037](0037-transitions-are-interruptible-dips.md) | Transitions: interruptible, a dip to black rather than a crossfade, instant under reduced motion | Accepted | Keep, and reconsider the crossfade | Done |
| [0038](0038-cursor-and-transient-overlays.md) | Hide the cursor; confirm switches with a toast only when nothing else names the input | Accepted | Keep | Done |
| [0039](0039-visual-language.md) | Visual language: SBP colours, system fonts only, sized for a lit room at a distance | Accepted | Keep | Partial |
| [0040](0040-remote-keyboard.md) | Forward the presenter's clicker to a networked keyboard | Accepted | Keep | Partial |

### Platform

| ADR | Decision | Status | egui verdict | egui status |
|---|---|---|---|---|
| [0041](0041-settings-json.md) | Settings are one JSON file in the app's data folder; network features default off; test mode never writes | Accepted | Keep, and fix | Done |
| [0042](0042-test-mode-flags.md) | Test-mode flags reach every state without hardware, and say so loudly | Accepted | Keep | Done |
| [0043](0043-keep-privileged-work-in-one-place.md) | Keep privileged work in one small, auditable place | Accepted | Drop the mechanism; keep the principle | N/A |
| [0044](0044-a-bounded-log-file-is-the-operator-contract.md) | A bounded daily log file is the operators' evidence | Accepted | Keep | Done |
| [0051](0051-request-capture-permission-up-front.md) | Ask for camera access before opening any capture | Accepted | Keep | Partial |
| [0052](0052-run-background-work-in-app-logic.md) | Run everything that must not stop in `App::logic`, not in `ui` | Accepted | Keep (egui-only decision) | Done |

### Delivery

| ADR | Decision | Status | egui verdict | egui status |
|---|---|---|---|---|
| [0045](0045-release-by-hand-bump-from-commits.md) | Release by hand; derive the version bump from Conventional Commits, with an explicit override | Accepted | Keep | Partial |
| [0046](0046-version-and-tagging.md) | VERSION is the source of truth; bump through a PR, tag the merged commit | Accepted | Adapt | Partial |
| [0047](0047-gate-releases-on-tests.md) | Gate every PR and release on lint, tests and build, and show the results inline | Accepted | Keep | Partial |
| [0048](0048-package-and-update-from-the-public-repo.md) | Package per platform and auto-update from the public source repo, asking first | Accepted | Keep; silent updates | Done (silent) |
| [0049](0049-builds-are-unsigned.md) | Builds are unsigned | Accepted (by default) | Keep unsigned | Same as Electron (ad-hoc on macOS) |
| [0050](0050-supply-chain-and-licence.md) | Supply chain and licence: SHA-pinned actions, delayed Dependabot, public registries, Apache-2.0 with SPDX headers | Accepted | Keep | Partial |

## Suggested port order

Driven by what keeps the wall working, not by size.

1. ~~**Gate: the wall itself.**~~ Passed 2026-10-12: both cards at 3840x2160,
   capture ~40 fps, UI 60 fps at full wall size
   ([0002](0002-port-to-rust-and-egui.md)).
2. **Capture robustness:** ~~health thresholds and reopen schedules~~
   (done, [0012](0012-judge-health-by-counting-frames.md),
   [0013](0013-reopen-by-releasing-the-device-with-two-backoffs.md)); native format choice ([0007](0007-capture-at-native-resolution.md)),
   hot-plug events ([0011](0011-choose-startup-inputs-by-precedence.md)); ~~the
   log file~~ (done, [0044](0044-a-bounded-log-file-is-the-operator-contract.md)).
   This is what actually recovers the wall today.
3. **Picture:** black-bar cropping ([0015](0015-crop-black-bars-by-snapping-to-shapes.md)),
   and its driver-query alternative (#340).
4. **Sound:** audio pairing and per-side volume
   ([0009](0009-pair-audio-with-its-video-card.md), [0010](0010-volume-belongs-to-a-side.md)).
5. **Deployment:** install location and startup script, unattended updates,
   single instance (D3-D5).
6. **Operator UI gaps:** cursor, transitions, remote keyboard, remaining
   Settings panes.
7. **Screensavers:** the six pure-fragment savers are done; next the
   orchestrated ones, with Art-Net and weather alongside.
8. **Reference detection:** wanted (D1), off the video path.
