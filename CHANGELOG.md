# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
