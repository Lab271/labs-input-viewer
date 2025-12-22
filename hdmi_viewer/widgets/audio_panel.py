"""Audio control panel widget."""

from PyQt6.QtCore import Qt, pyqtSignal
from PyQt6.QtWidgets import (
    QFrame,
    QHBoxLayout,
    QLabel,
    QPushButton,
    QSlider,
    QVBoxLayout,
)

from hdmi_viewer.log import Log
from hdmi_viewer.widgets.base import BUTTON_STYLE, HoverIcon, PANEL_STYLE, SLIDER_STYLE


class AudioPanel(QFrame):
    """Audio control panel with volume sliders and mute buttons."""

    # Signals
    input_volume_changed = pyqtSignal(int)  # volume 0-100
    system_volume_changed = pyqtSignal(int)  # volume 0-100
    input_mute_toggled = pyqtSignal(bool)  # muted
    system_mute_toggled = pyqtSignal(bool)  # muted

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setFixedSize(250, 180)
        self.setStyleSheet(f"""
            QFrame#audioPanel {{
                {PANEL_STYLE.replace('QFrame', '').replace('QLabel', '')}
            }}
            QLabel {{
                color: white;
                font-family: Arial, sans-serif;
                background: transparent;
            }}
            {SLIDER_STYLE}
        """)
        self.setObjectName("audioPanel")

        self._setup_ui()
        self.hide()

    def _setup_ui(self):
        """Set up the panel UI."""
        layout = QVBoxLayout(self)
        layout.setContentsMargins(15, 15, 15, 15)
        layout.setSpacing(12)

        # Title
        title = QLabel("♪ Audio Control")
        title.setStyleSheet("font-size: 14px; font-weight: bold; color: rgba(255, 255, 255, 200);")
        layout.addWidget(title)

        # Input audio (from capture card)
        input_row = QHBoxLayout()
        input_label = QLabel("← Input")
        input_label.setFixedWidth(60)
        self.input_volume_slider = QSlider(Qt.Orientation.Horizontal)
        self.input_volume_slider.setRange(0, 100)
        self.input_volume_slider.setValue(100)
        self.input_volume_slider.valueChanged.connect(self._on_input_volume_changed)
        self.input_volume_value = QLabel("100%")
        self.input_volume_value.setFixedWidth(40)
        input_row.addWidget(input_label)
        input_row.addWidget(self.input_volume_slider)
        input_row.addWidget(self.input_volume_value)
        layout.addLayout(input_row)

        # System audio (PC output)
        system_row = QHBoxLayout()
        system_label = QLabel("→ System")
        system_label.setFixedWidth(60)
        self.system_volume_slider = QSlider(Qt.Orientation.Horizontal)
        self.system_volume_slider.setRange(0, 100)
        self.system_volume_slider.setValue(100)
        self.system_volume_slider.valueChanged.connect(self._on_system_volume_changed)
        self.system_volume_value = QLabel("100%")
        self.system_volume_value.setFixedWidth(40)
        system_row.addWidget(system_label)
        system_row.addWidget(self.system_volume_slider)
        system_row.addWidget(self.system_volume_value)
        layout.addLayout(system_row)

        # Mute buttons
        mute_row = QHBoxLayout()
        self.input_mute_btn = QPushButton("⊘ Mute Input")
        self.input_mute_btn.setCheckable(True)
        self.input_mute_btn.setStyleSheet(BUTTON_STYLE)
        self.input_mute_btn.clicked.connect(self._on_input_mute_toggle)

        self.system_mute_btn = QPushButton("⊘ Mute System")
        self.system_mute_btn.setCheckable(True)
        self.system_mute_btn.setStyleSheet(BUTTON_STYLE)
        self.system_mute_btn.clicked.connect(self._on_system_mute_toggle)

        mute_row.addWidget(self.input_mute_btn)
        mute_row.addWidget(self.system_mute_btn)
        layout.addLayout(mute_row)

        layout.addStretch()

    def _on_input_volume_changed(self, value: int):
        """Handle input volume slider change."""
        self.input_volume_value.setText(f"{value}%")
        Log.debug(f"Input volume: {value}%")
        self.input_volume_changed.emit(value)

    def _on_system_volume_changed(self, value: int):
        """Handle system volume slider change."""
        self.system_volume_value.setText(f"{value}%")
        Log.debug(f"System volume: {value}%")
        self.system_volume_changed.emit(value)

    def _on_input_mute_toggle(self):
        """Toggle input audio mute."""
        muted = self.input_mute_btn.isChecked()
        self.input_mute_btn.setText("♪ Unmute Input" if muted else "⊘ Mute Input")
        Log.info(f"Input audio: {'MUTED' if muted else 'UNMUTED'}")
        self.input_mute_toggled.emit(muted)

    def _on_system_mute_toggle(self):
        """Toggle system audio mute."""
        muted = self.system_mute_btn.isChecked()
        self.system_mute_btn.setText("♪ Unmute System" if muted else "⊘ Mute System")
        Log.info(f"System audio: {'MUTED' if muted else 'UNMUTED'}")
        self.system_mute_toggled.emit(muted)


class AudioIcon(HoverIcon):
    """Audio icon button that toggles the AudioPanel."""

    def __init__(self, parent=None):
        super().__init__("♪", parent)
