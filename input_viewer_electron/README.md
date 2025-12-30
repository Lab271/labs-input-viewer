# Input Viewer - Electron App

A lightweight video input viewer built with Electron - OBS without the complexity.

## Features

- View and manage capture card feeds with a clean, simple interface
- Multi-input display (dual view or single view)
- Auto-updates via electron-updater
- Native installers for macOS (DMG), Windows (NSIS), and Linux (AppImage)
- Keyboard shortcuts for quick navigation
- No-signal detection with visual feedback

## Development

### Prerequisites

- Node.js 18+ 
- npm or yarn

### Setup

```bash
cd input_viewer_electron
npm install
```

### Run in Development

```bash
npm run dev
```

### Build for Production

```bash
# Build for current platform
npm run build

# Build for specific platform
npm run build:mac
npm run build:win
npm run build:linux
```

### Publish Release

Releases are published to GitHub when you push a tag:

```bash
git tag v2.0.0
git push origin v2.0.0
```

Or publish manually:

```bash
npm run publish
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `D` | Dual view (both feeds) |
| `L` | Single view: left feed |
| `R` | Single view: right feed |
| `1-4` | Select input directly |
| `F11` / `F` | Toggle fullscreen |
| `Escape` | Exit fullscreen |
| `Q` | Quit |

## Project Structure

```
input_viewer_electron/
├── package.json          # Electron app config and dependencies
├── build/                # Build resources (icons, entitlements)
├── src/
│   ├── main/
│   │   ├── main.js       # Main process (window management, IPC)
│   │   └── preload.js    # Preload script (secure API bridge)
│   └── renderer/
│       ├── index.html    # App HTML
│       ├── styles.css    # Styles
│       └── renderer.js   # Renderer process (UI logic, video capture)
└── dist/                 # Built executables (generated)
```

## Auto-Updates

The app uses `electron-updater` to check for updates from GitHub Releases. Updates are downloaded in the background and installed when the app quits.

To create a new release:

1. Update version in `package.json`
2. Commit and push changes
3. Create and push a git tag: `git tag v2.0.1 && git push origin v2.0.1`
4. GitHub Actions will build and publish the release
