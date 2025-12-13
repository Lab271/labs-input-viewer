# HDMI Viewer

Dual HDMI capture card viewer for ultrawide displays. Displays two camera feeds side by side with configurable spacing, designed for 6000x1200 resolution displays with Elgato Cam Link Pro.

## Features

- **Dual feed display** - Two HDMI inputs side by side
- **Layout switching** - Dual view or single feed centered
- **Camera switching** - Cycle through capture card inputs
- **No-signal detection** - Automatically detects disconnected sources
- **Test mode** - Animated mock sources for development without hardware
- **Fullscreen support** - Designed for dedicated display setups

## Installation

```bash
# Clone the repository
git clone https://github.com/LAB271/hdmi-viewer.git
cd hdmi-viewer

# Create virtual environment
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install dependencies
pip install -r requirements.txt
```

## Usage

### Production Mode (with real cameras)
```bash
python HDMI-viewer.py
```

### Test Mode (mock video sources)
```bash
python HDMI-viewer.py --test
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `D` | Dual view (both feeds) |
| `1` | Single view: left feed centered |
| `2` | Single view: right feed centered |
| `←` / `→` | Switch camera index (single view) |
| `Shift + ←/→` | Switch left camera (dual view) |
| `Ctrl + ←/→` | Switch right camera (dual view) |
| `F11` / `F` | Toggle fullscreen |
| `Escape` | Exit fullscreen (or quit if windowed) |
| `Q` | Quit |

## Configuration

Edit the configuration section in `HDMI-viewer.py`:

| Setting | Default | Description |
|---------|---------|-------------|
| `LEFT_CAMERA_INDEX` | 0 | Camera index for left display |
| `RIGHT_CAMERA_INDEX` | 1 | Camera index for right display |
| `AVAILABLE_CAMERA_INDICES` | [0,1,2,3] | Available camera inputs |
| `TARGET_WIDTH` | 1920 | Capture width |
| `TARGET_HEIGHT` | 1080 | Capture height |
| `TARGET_FPS` | 30 | Target frame rate |
| `SIDE_MARGIN` | 150 | Black space on edges (pixels) |
| `CENTER_GAP` | 200 | Space between feeds (pixels) |

## Project Structure

```
hdmi-viewer/
├── HDMI-viewer.py      # Main application
├── mock_sources.py     # Mock video sources for testing
├── requirements.txt    # Python dependencies
└── README.md
```

## Hardware Setup

Designed for:
- **Capture card**: Elgato Cam Link Pro (4x HDMI inputs)
- **Display**: Ultrawide monitor (6000x1200 resolution)
- **Platform**: Windows (also works on macOS/Linux)

### Platform-specific capture backends
- **Windows**: DirectShow (`cv2.CAP_DSHOW`)
- **macOS**: AVFoundation (`cv2.CAP_AVFOUNDATION`)
- **Linux**: V4L2 (`cv2.CAP_V4L2`)

## License

MIT License
