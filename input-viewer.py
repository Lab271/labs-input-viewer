#!/usr/bin/env python3
"""
Input Viewer - A lightweight video input viewer

This is a thin launcher for backwards compatibility.
The main application code is now in the input_viewer package.

Usage:
    python input-viewer.py                  # Production mode (real cameras)
    python input-viewer.py --mock           # Test mode (animated mock sources)
    python input-viewer.py --switch-signals # Cycle signal/no-signal every 10s
    python input-viewer.py --no-signal      # Always show no-signal overlay

Or use the new entry point:
    python -m input_viewer

Requirements:
    pip install PyQt6 opencv-python numpy Pillow
"""

from input_viewer import main

if __name__ == "__main__":
    main()
