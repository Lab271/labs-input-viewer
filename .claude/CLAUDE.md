# Input Viewer - Project Documentation

## Project Overview

Input Viewer is an Electron-based video input display application for viewing multiple video feeds simultaneously with no-signal detection capabilities. It's a lightweight alternative to OBS for simple input monitoring.

## Architecture

```
input_viewer_electron/
├── src/
│   ├── main/index.js          # Electron main process
│   ├── preload/index.js       # Context bridge (IPC)
│   └── renderer/
│       ├── index.html         # Main UI
│       ├── renderer.js        # Core app logic
│       ├── detection-simple.js # No-signal detection
│       └── styles.css         # Styling
├── build/                     # App icons and entitlements
├── package.json               # Dependencies and build config
└── electron.vite.config.mjs   # Build configuration
```

## Key Technologies

- **Electron** v35+ with electron-vite
- **electron-updater** for auto-updates from GitHub releases
- **WebRTC MediaDevices API** for video capture
- **Canvas API** for frame capture and comparison

## Development Commands

```bash
cd input_viewer_electron
npm run dev          # Start dev server with hot reload
npm run build        # Build for production
npm run build:mac    # Build macOS DMG
npm run build:win    # Build Windows installer
```

## Version Management

- **VERSION** file at repo root - source of truth for releases
- **package.json** version - must stay in sync (auto-release handles this)
- Tags follow semver: `v{major}.{minor}.{patch}`

## Commit Convention

Use [Conventional Commits](https://www.conventionalcommits.org/):

| Prefix | Description | Version Bump |
|--------|-------------|--------------|
| `feat:` | New feature | Minor (x.Y.0) |
| `fix:` | Bug fix | Patch (x.y.Z) |
| `perf:` | Performance improvement | Patch |
| `refactor:` | Code refactoring | Patch |
| `docs:` | Documentation only | None |
| `style:` | Code style (formatting) | None |
| `test:` | Tests only | None |
| `chore:` | Maintenance tasks | None |
| `build:` | Build system changes | Patch |
| `ci:` | CI/CD changes | Patch |
| `feat!:` or `BREAKING CHANGE` | Breaking change | Major (X.0.0) |

## Branch Naming

| Type | Pattern | Example |
|------|---------|---------|
| Feature | `feat/short-description` | `feat/add-dark-mode` |
| Bug fix | `fix/issue-description` | `fix/memory-leak-detection` |
| Performance | `perf/optimization-area` | `perf/frame-comparison` |
| Refactor | `refactor/area` | `refactor/settings-module` |
| Documentation | `docs/topic` | `docs/api-reference` |
| Chore | `chore/task` | `chore/update-deps` |

## Auto-Update Configuration

The app fetches updates from `LAB271/input-viewer-releases` (public repo).
Builds are published to both source repo and releases repo.

## CI/CD Flow

```
Feature Branch → PR → CI Tests → Merge to Main → Auto-Release → Build & Publish
                                      ↓
                              Analyzes commits
                              Bumps VERSION
                              Creates tag v*.*.*
                                      ↓
                              Builds macOS + Windows
                              Publishes to GitHub Releases
```

## Important Files

| File | Purpose |
|------|---------|
| `VERSION` | Current version (source of truth) |
| `.github/workflows/ci.yml` | PR and branch testing |
| `.github/workflows/auto-release.yml` | Version bumping and tagging |
| `.github/workflows/release.yml` | Building and publishing |

## Performance Considerations

The detection loop runs at ~1.6s intervals (every 100 frames at 60fps).
Key optimizations:
- Canvas context caching
- Pixel sampling (every 4th pixel)
- Debounced settings saves
- Conditional canvas resizing
