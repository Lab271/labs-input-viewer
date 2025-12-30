# Input Viewer

[![CI](https://github.com/LAB271/input-viewer/actions/workflows/ci.yml/badge.svg)](https://github.com/LAB271/input-viewer/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/LAB271/input-viewer)](https://github.com/LAB271/input-viewer/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A lightweight video input viewer — **OBS without the complexity**. View and manage capture card feeds with a clean, simple interface designed for users who need to display video inputs without the overhead of full streaming software.

## Versions

This repository contains two implementations:

| Version | Location | Stack | Status |
|---------|----------|-------|--------|
| **Electron** | [`input_viewer_electron/`](input_viewer_electron/) | Electron + JavaScript | **Recommended** |
| **Python** | [`input_viewer_python/`](input_viewer_python/) | PyQt6 + OpenCV | Legacy |

### Electron Version (Recommended)

The Electron version is faster, has native installers with auto-updates, and provides a smoother experience.

```bash
cd input_viewer_electron
npm install
npm start
```

See [input_viewer_electron/README.md](input_viewer_electron/README.md) for full details.

### Python Version

The original Python implementation using PyQt6 and OpenCV.

```bash
cd input_viewer_python
pip install -e .
python input-viewer.py
```

## Features

- **Multi-input display** — View one or two video feeds side by side
- **Layout switching** — Dual view or single feed centered (D/L/R keys)
- **Direct input selection** — Number keys 1-4 to switch inputs instantly
- **Settings panel** — Configure inputs with toggle switches
- **No-signal detection** — Custom animated overlay when source disconnects
- **Fullscreen support** — Designed for dedicated display setups
- **Any capture card** — Works with any video capture device
- **Auto-updater** — (Electron) Automatic updates from GitHub releases

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `D` | Dual view (both feeds) |
| `L` | Single view: left feed centered |
| `R` | Single view: right feed centered |
| `1-4` | Select input directly |
| `F11` / `F` | Toggle fullscreen |
| `Escape` | Exit fullscreen (or quit if windowed) |
| `Q` | Quit |

Hover over the ⓘ icon in the top-left corner for the shortcuts panel.

## Configuration

### Settings Panel

Click the ⚙ gear icon in the top-right corner to open the settings panel:

- **Toggle inputs** on/off
- **Set default input** (shown at startup)
- **Rename inputs** for easy identification
- Changes are saved automatically and applied in real-time

### settings.json

Settings are stored in `settings.json`:

```json
{
    "inputs": [
        {"index": 0, "name": "Laptop", "enabled": true, "default": true},
        {"index": 1, "name": "Desktop", "enabled": true, "default": false},
        {"index": 2, "name": "Input 3", "enabled": false, "default": false},
        {"index": 3, "name": "Input 4", "enabled": false, "default": false}
    ]
}
```

## Development

### Electron

```bash
cd input_viewer_electron
npm install
npm start          # Development mode
npm run build      # Build installers
```

### Python

```bash
cd input_viewer_python
pip install -e ".[dev]"
make test
make lint
```

## Hardware Setup

Works with:
- **Any capture card** — USB or PCIe capture devices
- **Any display** — Adapts to your screen resolution
- **Platforms**: macOS, Windows

## License

[MIT License](LICENSE) © 2025 LAB271
