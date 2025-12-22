"""Widget components for HDMI Viewer."""

from hdmi_viewer.widgets.toggle_switch import ToggleSwitch
from hdmi_viewer.widgets.overlays import InputNameOverlay, InfoPanel, ScreenSaver
from hdmi_viewer.widgets.settings_panel import SettingsPanel
from hdmi_viewer.widgets.audio_panel import AudioPanel
from hdmi_viewer.widgets.thumbnails import ThumbnailsPanel

__all__ = [
    "ToggleSwitch",
    "InputNameOverlay",
    "InfoPanel",
    "ScreenSaver",
    "SettingsPanel",
    "AudioPanel",
    "ThumbnailsPanel",
]
