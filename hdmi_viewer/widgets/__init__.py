"""Widget components for HDMI Viewer."""

from hdmi_viewer.widgets.audio_panel import AudioIcon, AudioPanel
from hdmi_viewer.widgets.base import HoverIcon
from hdmi_viewer.widgets.overlays import InfoIcon, InfoPanel, InputNameOverlay, ScreenSaver
from hdmi_viewer.widgets.settings_panel import SettingsPanel
from hdmi_viewer.widgets.thumbnails import ThumbnailsPanel
from hdmi_viewer.widgets.toggle_switch import ToggleSwitch

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
