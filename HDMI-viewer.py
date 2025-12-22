#!/usr/bin/env python3
"""
Dual Elgato Capture Card Viewer for Ultrawide Displays

This is a thin launcher for backwards compatibility.
The main application code is now in the hdmi_viewer package.

Usage:
    python HDMI-viewer.py                  # Production mode (real cameras)
    python HDMI-viewer.py --mock           # Test mode (animated mock sources)
    python HDMI-viewer.py --switch-signals # Cycle signal/no-signal every 10s
    python HDMI-viewer.py --no-signal      # Always show no-signal overlay

Or use the new entry point:
    python -m hdmi_viewer

Requirements:
    pip install PyQt6 opencv-python numpy Pillow
"""

from hdmi_viewer import main

if __name__ == "__main__":
    main()
