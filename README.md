# Input Viewer

[![CI](https://github.com/LAB271/input-viewer/actions/workflows/ci.yml/badge.svg)](https://github.com/LAB271/input-viewer/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/LAB271/input-viewer)](https://github.com/LAB271/input-viewer/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Python 3.10+](https://img.shields.io/badge/python-3.10+-blue.svg)](https://www.python.org/downloads/)

A lightweight video input viewer — **OBS without the complexity**. View and manage capture card feeds with a clean, simple interface designed for users who need to display video inputs without the overhead of full streaming software.

## Features

- **Multi-input display** — View one or two video feeds side by side
- **Layout switching** — Dual view or single feed centered (D/L/R keys)
- **Direct input selection** — Number keys 1-4 to switch inputs instantly
- **Settings panel** — Configure inputs with toggle switches
- **No-signal detection** — Custom animated overlay when source disconnects
- **Live reload** — Settings update without restarting the app
- **Test modes** — Mock sources for development without hardware
- **Fullscreen support** — Designed for dedicated display setups
- **Any capture card** — Works with any video capture device

## Installation

### Option 1: Download Release (Recommended)

Download the latest release for your platform from the [Releases page](https://github.com/LAB271/input-viewer/releases/latest):

- **macOS**: `input-viewer-macos`
- **Windows**: `input-viewer-windows.exe`
- **Linux**: `input-viewer-linux`

### Option 2: Run from Source

```bash
# Clone the repository
git clone https://github.com/LAB271/input-viewer.git
cd input-viewer

# Create virtual environment
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install dependencies
make install
# Or manually: pip install PyQt6 opencv-python numpy Pillow

# Run the application
make run
# Or: python input-viewer.py
```

## Usage

### Production Mode
```bash
python input-viewer.py              # Real camera inputs
python input-viewer.py --verbose    # With debug logging
```

### Test Mode
```bash
python input-viewer.py --mock           # Animated mock sources
python input-viewer.py --no-signal      # Always show no-signal overlay
python input-viewer.py --switch-signals # Cycle signal/no-signal every 10s
```

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

```bash
# Install dev dependencies
make install-dev

# Run tests
make test

# Run linter
make lint

# Format code
make format

# Run all checks
make check
```

## Hardware Setup

Works with:
- **Any capture card** — USB or PCIe capture devices
- **Any display** — Adapts to your screen resolution
- **Platform**: macOS, Windows, Linux

## Releasing

To create a new release:

```bash
# Update VERSION file
echo "1.1.0" > VERSION

# Create and push tag
git tag -a v1.1.0 -m "Release v1.1.0"
git push origin v1.1.0
```

The GitHub Actions workflow will automatically:
1. Run tests on all platforms
2. Build executables for macOS, Windows, and Linux
3. Create a GitHub Release with the binaries

## License

[MIT License](LICENSE) © 2025 LAB271
