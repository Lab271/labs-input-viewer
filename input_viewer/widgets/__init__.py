"""Widget components for Input Viewer."""

from input_viewer.widgets.audio_panel import AudioIcon, AudioPanel
from input_viewer.widgets.base import HoverIcon
from input_viewer.widgets.overlays import InfoIcon, InfoPanel, InputNameOverlay, ScreenSaver
from input_viewer.widgets.settings_panel import SettingsPanel
from input_viewer.widgets.thumbnails import ThumbnailsPanel
from input_viewer.widgets.toggle_switch import ToggleSwitch

__all__ = [
    "HoverIcon",
    "ToggleSwitch",
    "InputNameOverlay",
    "InfoPanel",
    "InfoIcon",
    "ScreenSaver",
    "SettingsPanel",
    "AudioPanel",
    "AudioIcon",
    "ThumbnailsPanel",
]
