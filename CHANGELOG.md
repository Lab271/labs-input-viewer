# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.5.2] - 2024-12-24

### Fixed

- Fixed Windows dual screen not working when both feeds use same input
- Mirror left feed to right display when using same camera input (avoids Windows camera lock)

## [1.5.1] - 2024-12-24

### Fixed

- Fixed Windows crash when switching cameras (QImage memory issue)
- Fixed no-signal animation not showing on Windows (missing mp4 in bundle)
- Added fallback camera backend on Windows if DirectShow fails
- Added error handling in camera worker thread to prevent freezes
- Added icon.ico to Windows bundle

## [1.5.0] - 2024-12-24

### Added

- Display Settings panel with configurable options:
  - Screensaver delay (10-300 seconds)
  - Cursor hide delay (1-30 seconds)
  - Side margin (0-500 pixels)
  - Center gap (0-500 pixels)
- Reset to Defaults button for display settings
- Settings are saved to settings.json and applied in real-time

### Changed

- Reduced default capture resolution from 4K to 1080p for better performance

## [1.4.6] - 2024-12-24

### Fixed

- Fixed mouse shake detection to properly reveal cursor when hidden
- Mouse tracking now enabled on all child widgets for reliable event capture
- Improved shake detection algorithm with better direction reversal tracking

### Added

- Screensaver now exits on mouse shake (in addition to showing cursor)
- Screensaver now exits on any keyboard input

## [1.4.5] - 2024-12-24

### Fixed

- Fixed PyInstaller spec file asset paths (assets now bundled from `assets/` folder)

## [1.4.3] - 2024-12-24

### Fixed

- Updated PyInstaller spec file with correct module references

## [1.4.0] - 2024-12-24

### Added
- Threaded camera capture with `CameraWorker` for non-blocking frame reads
- `HoverIcon` base class for reusable icon widgets with hover effects
- Shared stylesheet constants in `widgets/base.py`
- Architecture documentation (`hdmi_viewer/ARCHITECTURE.md`)
- Feature testing checklist (`TESTING.md`)

### Changed
- Camera feeds now run in background threads (~60fps capture)
- UI timer only handles display, not blocking I/O
- Refactored `SettingsIcon`, `AudioIcon`, `InfoIcon` to use `HoverIcon` base
- Simplified `ScreenSaver` into smaller focused methods
- Reduced code duplication in input switching logic
- Cleaned up `ToggleSwitch` - removed deprecated callback

### Performance
- Smoother frame rate due to parallel camera capture
- Reduced UI blocking during frame reads
- Better CPU utilization with threaded workers

## [1.3.0] - 2025-12-16

### Changed
- Renamed application to "Space Presenter"
- Updated capture resolution to 4K (3840x2160) at 30Hz
- Added "By Labs for _Space" attribution in info panel

## [1.2.0] - 2025-12-16

### Changed
- Replaced no-signal detection with vision model using multi-vector feature extraction
- No-signal detection now compares against reference image (elgato_no_source.png)
- Uses cosine similarity on color histograms, spatial intensity, edge density, and statistical features
- Only checks every 100 frames (~3.3s at 30Hz) for efficiency
- Fixes false positives on dark/grey applications and presentations

### Improved
- Mock no-signal mode now cycles through 3 test screens: elgato_no_source.png, zed.png, and uniform grey

## [1.0.0] - 2025-12-13

### Added
- Dual HDMI feed display for ultrawide monitors (6000x1200)
- Layout modes: Dual view, Single left, Single right
- Direct input selection with number keys 1-4
- Settings panel with toggle switches for input configuration
- Live settings reload without app restart
- Custom no-signal animation with animated HDMI cable
- Verbose logging mode with colored output
- Test modes: mock sources, signal cycling, always no-signal
- Fullscreen support
- Input name overlay when switching inputs
- Keyboard shortcuts info panel

### Configuration
- `settings.json` for input configuration (name, enabled, default)
- Customizable side margins and center gap
- Support for Elgato Cam Link Pro (4 inputs)

### Keyboard Shortcuts
- `D` - Dual view
- `L` - Single left view
- `R` - Single right view
- `1-4` - Select input directly
- `F11/F` - Toggle fullscreen
- `Q` - Quit

[1.0.0]: https://github.com/LAB271/hdmi-viewer/releases/tag/v1.0.0
