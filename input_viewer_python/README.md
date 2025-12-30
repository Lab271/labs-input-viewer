# Input Viewer - Python Version

The original Python implementation of Input Viewer using PyQt6 and OpenCV.

> **Note**: The [Electron version](../input_viewer_electron/) is now recommended for better performance and easier distribution.

## Installation

### From Source

```bash
# Create virtual environment
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install dependencies
pip install -e .

# Or with uv:
uv sync
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

## Development

```bash
# Install dev dependencies
pip install -e ".[dev]"

# Run tests
make test

# Run linter
make lint

# Format code
make format

# Run all checks
make check
```

## Building Executables

```bash
# Install PyInstaller
pip install pyinstaller

# Build executable
pyinstaller input-viewer.spec
```

The built executable will be in the `dist/` folder.

## Architecture

See [input_viewer/ARCHITECTURE.md](input_viewer/ARCHITECTURE.md) for details on the codebase structure.
