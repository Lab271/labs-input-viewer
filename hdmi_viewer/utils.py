"""
Utility functions for resource and path handling.
"""

import os
import sys


def get_resource_path(relative_path: str) -> str:
    """Get absolute path to resource, works for dev and for PyInstaller bundle."""
    if hasattr(sys, "_MEIPASS"):
        # PyInstaller creates a temp folder and stores path in _MEIPASS
        base_path = sys._MEIPASS
    else:
        # Development mode - use package directory
        base_path = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    
    # Check assets folder first, then root
    assets_path = os.path.join(base_path, "assets", relative_path)
    if os.path.exists(assets_path):
        return assets_path
    return os.path.join(base_path, relative_path)


def get_user_data_path(filename: str) -> str:
    """Get path for user data files (settings that need to persist)."""
    if hasattr(sys, "_MEIPASS"):
        # When bundled, use user's home directory for writable files
        if sys.platform == "darwin":
            data_dir = os.path.expanduser("~/Library/Application Support/Space Presenter")
        elif sys.platform == "win32":
            data_dir = os.path.join(os.environ.get("APPDATA", ""), "Space Presenter")
        else:
            data_dir = os.path.expanduser("~/.config/hdmi-viewer")
        os.makedirs(data_dir, exist_ok=True)
        return os.path.join(data_dir, filename)
    else:
        # Development mode - use project root directory
        return os.path.join(
            os.path.dirname(os.path.dirname(os.path.abspath(__file__))), filename
        )
