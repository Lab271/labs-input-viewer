# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## Electron Version (v2.x)

### [2.2.1] - 2026-01-23

#### Fixed

- Build artifacts now have correct version in filenames

### [2.2.0] - 2026-01-19

#### Added

- DVD-style bouncing logo screensaver when feeds lose signal
  - Bouncing logo animation with color changes on bounce
  - 5-minute delay before screensaver activates
  - Activates when all feeds show no-signal

#### Changed

- Consolidated CI/CD into single workflow
- Improved release automation

### [2.1.13] - 2026-01-16

#### Fixed

- Center divider logo now constrained to fit within gap

### [2.1.12] - 2026-01-16

#### Fixed

- Center divider logo scales to fit height on wide screens

### [2.1.11] - 2026-01-16

#### Fixed

- Video capture retries at device max resolution if initial resolution is low

### [2.1.10] - 2026-01-16

#### Fixed

- Request exact 1920x1200 resolution from capture device

### [2.1.9] - 2026-01-16

#### Fixed

- Use min constraints to force higher video capture resolution

### [2.1.8] - 2026-01-16

#### Fixed

- Enable hardware acceleration for better video quality

### [2.1.7] - 2026-01-16

#### Fixed

- Improved video capture quality with higher resolution constraints

### [2.1.5] - 2026-01-16

#### Fixed

- Prevent video cropping in single view mode

### [2.1.4] - 2026-01-16

#### Changed

- Streamlined CI/CD pipeline

### [2.1.0] - 2026-01-16

#### Added

- Automatic releases triggered by conventional commits
- Version bumping based on commit types (feat/fix/etc.)

### [2.0.0] - 2026-01-11

#### Added

- Complete rewrite using Electron + electron-vite
- Native installers for macOS (.dmg) and Windows (.exe)
- Auto-updater with GitHub releases integration
- Dual/Single view modes with keyboard shortcuts
- Freeze frame functionality (Space key)
- Settings panel with input configuration
- No-signal detection with reference screenshot capture
- Center gap and border width sliders
- Logo overlay in center divider and single view mode
- Dropdown panel for settings access

#### Changed

- Switched from Python/PyQt6 to JavaScript/Electron
- Improved performance with WebRTC MediaDevices API
- Cleaner UI with modern styling

---

## Python Version (v1.x) - Legacy

### [1.5.3] - 2025-12-24

#### Added

- File logging for bundled apps to help debug crashes
- Global exception handler to catch and log crashes with full stack traces
- Log file location: `%APPDATA%\Input Viewer\app.log` (Windows)

### [1.5.2] - 2025-12-24

#### Fixed

- Fixed Windows dual screen not working when both feeds use same input
- Mirror left feed to right display when using same camera input

### [1.5.1] - 2025-12-24

#### Fixed

- Fixed Windows crash when switching cameras (QImage memory issue)
- Fixed no-signal animation not showing on Windows (missing mp4 in bundle)
- Added fallback camera backend on Windows if DirectShow fails
- Added error handling in camera worker thread to prevent freezes
- Added icon.ico to Windows bundle

### [1.5.0] - 2025-12-24

#### Added

- Display Settings panel with configurable options:
  - Screensaver delay (10-300 seconds)
  - Cursor hide delay (1-30 seconds)
  - Side margin (0-500 pixels)
  - Center gap (0-500 pixels)
- Reset to Defaults button for display settings
- Settings are saved to settings.json and applied in real-time

#### Changed

- Reduced default capture resolution from 4K to 1080p for better performance

### [1.4.6] - 2025-12-24

#### Fixed

- Fixed mouse shake detection to properly reveal cursor when hidden
- Mouse tracking now enabled on all child widgets for reliable event capture
- Improved shake detection algorithm with better direction reversal tracking

#### Added

- Screensaver now exits on mouse shake (in addition to showing cursor)
- Screensaver now exits on any keyboard input

### [1.4.0] - 2025-12-24

#### Added

- Threaded camera capture with `CameraWorker` for non-blocking frame reads
- `HoverIcon` base class for reusable icon widgets with hover effects
- Architecture documentation (`input_viewer/ARCHITECTURE.md`)

#### Changed

- Camera feeds now run in background threads (~60fps capture)
- UI timer only handles display, not blocking I/O

#### Performance

- Smoother frame rate due to parallel camera capture
- Reduced UI blocking during frame reads
- Better CPU utilization with threaded workers

### [1.3.0] - 2025-12-16

#### Changed

- Updated capture resolution to 4K (3840x2160) at 30Hz
- Added attribution in info panel

### [1.2.0] - 2025-12-16

#### Changed

- Replaced no-signal detection with vision model using multi-vector feature extraction
- No-signal detection now compares against reference image
- Uses cosine similarity on color histograms, spatial intensity, edge density, and statistical features

### [1.0.0] - 2025-12-13

#### Added

- Multi-input video feed display
- Layout modes: Dual view, Single left, Single right
- Direct input selection with number keys 1-4
- Settings panel with toggle switches for input configuration
- Live settings reload without app restart
- Custom no-signal animation
- Fullscreen support
- Input name overlay when switching inputs
- Keyboard shortcuts info panel

[2.2.1]: https://github.com/LAB271/input-viewer/releases/tag/v2.2.1
[2.2.0]: https://github.com/LAB271/input-viewer/releases/tag/v2.2.0
[2.1.13]: https://github.com/LAB271/input-viewer/releases/tag/v2.1.13
[2.1.12]: https://github.com/LAB271/input-viewer/releases/tag/v2.1.12
[2.1.11]: https://github.com/LAB271/input-viewer/releases/tag/v2.1.11
[2.1.10]: https://github.com/LAB271/input-viewer/releases/tag/v2.1.10
[2.1.9]: https://github.com/LAB271/input-viewer/releases/tag/v2.1.9
[2.1.8]: https://github.com/LAB271/input-viewer/releases/tag/v2.1.8
[2.1.7]: https://github.com/LAB271/input-viewer/releases/tag/v2.1.7
[2.1.5]: https://github.com/LAB271/input-viewer/releases/tag/v2.1.5
[2.1.4]: https://github.com/LAB271/input-viewer/releases/tag/v2.1.4
[2.1.0]: https://github.com/LAB271/input-viewer/releases/tag/v2.1.0
[2.0.0]: https://github.com/LAB271/input-viewer/releases/tag/v2.0.0
[1.5.3]: https://github.com/LAB271/input-viewer/releases/tag/v1.5.3
[1.5.2]: https://github.com/LAB271/input-viewer/releases/tag/v1.5.2
[1.5.1]: https://github.com/LAB271/input-viewer/releases/tag/v1.5.1
[1.5.0]: https://github.com/LAB271/input-viewer/releases/tag/v1.5.0
[1.4.6]: https://github.com/LAB271/input-viewer/releases/tag/v1.4.6
[1.4.0]: https://github.com/LAB271/input-viewer/releases/tag/v1.4.0
[1.3.0]: https://github.com/LAB271/input-viewer/releases/tag/v1.3.0
[1.2.0]: https://github.com/LAB271/input-viewer/releases/tag/v1.2.0
[1.0.0]: https://github.com/LAB271/input-viewer/releases/tag/v1.0.0
