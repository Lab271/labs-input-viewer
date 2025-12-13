# HDMI Viewer

[![CI](https://github.com/LAB271/hdmi-viewer/actions/workflows/ci.yml/badge.svg)](https://github.com/LAB271/hdmi-viewer/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/LAB271/hdmi-viewer)](https://github.com/LAB271/hdmi-viewer/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Python 3.10+](https://img.shields.io/badge/python-3.10+-blue.svg)](https://www.python.org/downloads/)

Dual HDMI capture card viewer for ultrawide displays. Displays two camera feeds side by side with configurable spacing, designed for 6000x1200 resolution displays with Elgato Cam Link Pro.

## Features

- **Dual feed display** — Two HDMI inputs side by side
- **Layout switching** — Dual view or single feed centered (D/L/R keys)
- **Direct input selection** — Number keys 1-4 to switch inputs instantly
- **Settings panel** — Configure inputs with toggle switches
- **No-signal detection** — Custom animated overlay when source disconnects
- **Live reload** — Settings update without restarting the app
- **Test modes** — Mock sources for development without hardware
- **Fullscreen support** — Designed for dedicated display setups

## Installation

### Option 1: Download Release (Recommended)

Download the latest release for your platform from the [Releases page](https://github.com/LAB271/hdmi-viewer/releases/latest):

- **macOS**: `hdmi-viewer-macos`
- **Windows**: `hdmi-viewer-windows.exe`
- **Linux**: `hdmi-viewer-linux`

### Option 2: Run from Source

```bash
# Clone the repository
git clone https://github.com/LAB271/hdmi-viewer.git
cd hdmi-viewer

# Create virtual environment
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install dependencies
make install
# Or manually: pip install PyQt6 opencv-python numpy Pillow

# Run the application
make run
# Or: python HDMI-viewer.py
```

## Usage

### Production Mode
```bash
python HDMI-viewer.py              # Real camera inputs
python HDMI-viewer.py --verbose    # With debug logging
```

### Test Mode
```bash
python HDMI-viewer.py --mock           # Animated mock sources
python HDMI-viewer.py --no-signal      # Always show no-signal overlay
python HDMI-viewer.py --switch-signals # Cycle signal/no-signal every 10s
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
        {"index": 0, "name": "HDMI-Cable", "enabled": true, "default": true},
        {"index": 1, "name": "Apple TV", "enabled": true, "default": false},
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

Designed for:
- **Capture card**: Elgato Cam Link Pro (4× HDMI inputs)
- **Display**: Ultrawide monitor (6000×1200 resolution)
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
