# Input Viewer

[![CI/CD](https://github.com/LAB271/input-viewer/actions/workflows/ci.yml/badge.svg)](https://github.com/LAB271/input-viewer/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/LAB271/input-viewer)](https://github.com/LAB271/input-viewer/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A lightweight video input viewer — **OBS without the complexity**. View and manage capture card feeds with a clean, simple interface designed for users who need to display video inputs without the overhead of full streaming software.

## Download

Download the latest release for your platform:

- **macOS**: [Input Viewer.dmg](https://github.com/LAB271/input-viewer-releases/releases/latest)
- **Windows**: [Input Viewer Setup.exe](https://github.com/LAB271/input-viewer-releases/releases/latest)

The app includes auto-updates and will notify you when new versions are available.

## Features

- **Multi-input display** — View one or two video feeds side by side
- **Layout switching** — Dual view or single feed centered
- **Direct input selection** — Number keys 1-4 to switch inputs instantly
- **Freeze frame** — Pause any feed with Space key
- **Settings panel** — Configure inputs with toggle switches
- **No-signal detection** — Custom overlay when source disconnects
- **DVD screensaver** — Bouncing logo when feeds lose signal
- **Fullscreen support** — Designed for dedicated display setups
- **Auto-updater** — Automatic updates from GitHub releases
- **Any capture card** — Works with any video capture device

## Keyboard Shortcuts

| Key         | Action                              |
| ----------- | ----------------------------------- |
| `D`         | Dual view (both feeds)              |
| `S`         | Single view (selected feed centered)|
| `1-4`       | Select input directly               |
| `Space`     | Freeze/unfreeze current feed        |
| `F11` / `F` | Toggle fullscreen                   |
| `Escape`    | Exit fullscreen                     |
| `Q`         | Quit                                |

Hover over the top edge to reveal the settings dropdown panel.

## Configuration

### Settings Panel

Click the ⚙ gear icon to open the settings panel:

- **Toggle inputs** on/off
- **Set default input** (shown at startup)
- **Rename inputs** for easy identification
- **Adjust center gap** between feeds
- **Adjust border width** on sides
- Changes are saved automatically

### settings.json

Settings are stored in the app's user data directory:

```json
{
  "inputs": [
    {"index": 0, "name": "Laptop", "enabled": true, "default": true},
    {"index": 1, "name": "Desktop", "enabled": true, "default": false},
    {"index": 2, "name": "Input 3", "enabled": false, "default": false},
    {"index": 3, "name": "Input 4", "enabled": false, "default": false}
  ],
  "centerGap": 100,
  "borderWidth": 50
}
```

## Development

### Prerequisites

- Node.js 20+
- npm

### Setup

```bash
cd input_viewer_electron
npm install
npm run dev     # Development mode with hot reload
npm run build   # Build for production
```

### Building Installers

```bash
npm run build:mac   # Build macOS DMG
npm run build:win   # Build Windows installer
```

## Hardware

Works with:

- **Any capture card** — USB or PCIe capture devices
- **Any display** — Adapts to your screen resolution
- **Platforms**: macOS, Windows

## Legacy Python Version

The original Python implementation remains available in the [`v2.5.3` source tree](https://github.com/Lab271/input-viewer/tree/v2.5.3/input_viewer_python) but is no longer maintained. Use the Electron version for the best experience.

## License

[MIT License](LICENSE) © 2025 LAB271
