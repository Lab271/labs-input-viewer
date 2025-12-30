"""
Configuration management for Input Viewer.

Handles settings loading/saving, input configuration, and application constants.
"""

import json
import os
import shutil
from dataclasses import dataclass
from enum import Enum, auto

from input_viewer.log import Log
from input_viewer.utils import get_resource_path, get_user_data_path


class LayoutMode(Enum):
    """Available layout modes for the viewer."""

    DUAL = auto()  # Both feeds side by side
    SINGLE_LEFT = auto()  # Only left feed, centered
    SINGLE_RIGHT = auto()  # Only right feed, centered


@dataclass
class InputConfig:
    """Configuration for a single input."""

    index: int
    name: str
    enabled: bool = True
    default: bool = False


# =============================================================================
# CONFIGURATION CONSTANTS
# =============================================================================

# Capture settings
TARGET_WIDTH = 1920
TARGET_HEIGHT = 1080
TARGET_FPS = 30

# Layout spacing (in pixels)
SIDE_MARGIN = 150  # Black space on left and right edges
CENTER_GAP = 200  # Black space between the two feeds

# Logo settings
LOGO_FILENAME = "logo.png"  # Logo file in the assets folder

# No signal message
NO_SIGNAL_MESSAGE = "No signal detected"

# Display settings
FULLSCREEN = True
TIMER_INTERVAL_MS = 30

# Cursor auto-hide delay
CURSOR_HIDE_DELAY_MS = 3000


# =============================================================================
# SETTINGS MANAGEMENT
# =============================================================================


def load_settings() -> dict:
    """Load settings from settings.json file."""
    settings_path = get_user_data_path("settings.json")

    # If no user settings exist, copy default from bundle
    if not os.path.exists(settings_path):
        default_settings_path = get_resource_path("settings.json")
        if os.path.exists(default_settings_path):
            try:
                shutil.copy(default_settings_path, settings_path)
                Log.debug(f"Copied default settings to {settings_path}")
            except OSError:
                pass

    if os.path.exists(settings_path):
        try:
            with open(settings_path) as f:
                settings = json.load(f)
            Log.debug(f"Settings loaded from {settings_path}")
            return settings
        except (OSError, json.JSONDecodeError) as e:
            Log.warning(f"Failed to load settings.json: {e}")

    # Return default settings if file doesn't exist or fails to load
    return {
        "inputs": [
            {"index": 0, "name": "Input 1", "enabled": True, "default": True},
            {"index": 1, "name": "Input 2", "enabled": True, "default": False},
            {"index": 2, "name": "Input 3", "enabled": True, "default": False},
            {"index": 3, "name": "Input 4", "enabled": False, "default": False},
        ],
        "display": {
            "screensaver_delay": 60,
            "cursor_hide_delay": 3,
            "side_margin": 150,
            "center_gap": 200,
        }
    }


def save_settings(settings: dict):
    """Save settings to settings.json file."""
    settings_path = get_user_data_path("settings.json")

    try:
        with open(settings_path, "w") as f:
            json.dump(settings, f, indent=4)
        Log.debug(f"Settings saved to {settings_path}")
    except OSError as e:
        Log.error(f"Failed to save settings.json: {e}")


def get_input_configs(settings: dict) -> list[InputConfig]:
    """Parse input configurations from settings."""
    inputs = []
    for input_data in settings.get("inputs", []):
        inputs.append(
            InputConfig(
                index=input_data.get("index", 0),
                name=input_data.get("name", f"Input {input_data.get('index', 0)}"),
                enabled=input_data.get("enabled", True),
                default=input_data.get("default", False),
            )
        )
    return inputs


def get_enabled_inputs(inputs: list[InputConfig]) -> list[InputConfig]:
    """Get list of enabled inputs."""
    return [inp for inp in inputs if inp.enabled]


def get_display_settings() -> dict:
    """Get display settings with defaults."""
    settings = load_settings()
    display = settings.get("display", {})
    return {
        "screensaver_delay": display.get("screensaver_delay", 60),
        "cursor_hide_delay": display.get("cursor_hide_delay", 3),
        "side_margin": display.get("side_margin", 150),
        "center_gap": display.get("center_gap", 200),
    }


def get_default_input(inputs: list[InputConfig]) -> InputConfig | None:
    """Get the default input, or first enabled input if no default."""
    for inp in inputs:
        if inp.default and inp.enabled:
            return inp
    # Fallback to first enabled input
    enabled = get_enabled_inputs(inputs)
    return enabled[0] if enabled else None


# =============================================================================
# GLOBAL STATE (initialized on import)
# =============================================================================

# These are loaded once at startup and can be refreshed via reload_config()
_settings: dict = {}
_input_configs: list[InputConfig] = []
_enabled_inputs: list[InputConfig] = []
_default_input: InputConfig | None = None
_available_input_indices: list[int] = []


def _init_config():
    """Initialize configuration from settings file."""
    global _settings, _input_configs, _enabled_inputs, _default_input, _available_input_indices

    _settings = load_settings()
    _input_configs = get_input_configs(_settings)
    _enabled_inputs = get_enabled_inputs(_input_configs)
    _default_input = get_default_input(_input_configs)
    _available_input_indices = [inp.index for inp in _enabled_inputs]


def reload_config():
    """Reload configuration from settings file."""
    _init_config()


def get_settings() -> dict:
    """Get current settings dict."""
    return _settings


def get_all_input_configs() -> list[InputConfig]:
    """Get all input configurations."""
    return _input_configs


def get_all_enabled_inputs() -> list[InputConfig]:
    """Get list of enabled inputs."""
    return _enabled_inputs


def get_current_default_input() -> InputConfig | None:
    """Get the default input."""
    return _default_input


def get_available_input_indices() -> list[int]:
    """Get list of available (enabled) input indices."""
    return _available_input_indices


def get_left_input_index() -> int:
    """Get the default left input index."""
    return _default_input.index if _default_input else 0


def get_right_input_index() -> int:
    """Get the default right input index."""
    return _default_input.index if _default_input else 0


# Initialize config on module import
_init_config()
