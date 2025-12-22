"""
Audio control panel widget.
"""

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
        self.setStyleSheet("""
            QFrame#audioPanel {
                background-color: rgba(30, 30, 30, 240);
                border-radius: 12px;
                border: 1px solid rgba(255, 255, 255, 30);
            }
            QLabel {
                color: white;
                font-family: Arial, sans-serif;
                background: transparent;
            }
            QSlider::groove:horizontal {
                height: 6px;
                background: rgba(255, 255, 255, 30);
                border-radius: 3px;
            }
            QSlider::handle:horizontal {
                width: 16px;
                height: 16px;
                margin: -5px 0;
                background: white;
                border-radius: 8px;
            }
            QSlider::sub-page:horizontal {
                background: #58a6ff;
                border-radius: 3px;
            }
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
        self.input_mute_btn.setStyleSheet("""
            QPushButton {
                background-color: rgba(60, 60, 60, 200);
                color: white;
                border: 1px solid rgba(255, 255, 255, 30);
                border-radius: 6px;
                padding: 6px 12px;
                font-size: 11px;
            }
            QPushButton:hover {
                background-color: rgba(80, 80, 80, 200);
            }
            QPushButton:checked {
                background-color: rgba(255, 59, 48, 200);
            }
        """)
        self.input_mute_btn.clicked.connect(self._on_input_mute_toggle)

        self.system_mute_btn = QPushButton("⊘ Mute System")
        self.system_mute_btn.setCheckable(True)
        self.system_mute_btn.setStyleSheet("""
            QPushButton {
                background-color: rgba(60, 60, 60, 200);
                color: white;
                border: 1px solid rgba(255, 255, 255, 30);
                border-radius: 6px;
                padding: 6px 12px;
                font-size: 11px;
            }
            QPushButton:hover {
                background-color: rgba(80, 80, 80, 200);
            }
            QPushButton:checked {
                background-color: rgba(255, 59, 48, 200);
            }
        """)
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


class AudioIcon(QLabel):
    """Audio icon button that toggles the AudioPanel."""

    # Signal emitted when clicked
    clicked = pyqtSignal()

    def __init__(self, parent=None):
        super().__init__("♪", parent)
        self.setStyleSheet("""
            QLabel {
                color: rgba(255, 255, 255, 100);
                font-size: 28px;
                background: transparent;
            }
        """)
        self.adjustSize()
        self.setCursor(Qt.CursorShape.PointingHandCursor)

    def mousePressEvent(self, event):
        """Handle click."""
        self.clicked.emit()

    def enterEvent(self, event):
        """Highlight on hover."""
        self.setStyleSheet("""
            QLabel {
                color: rgba(255, 255, 255, 255);
                font-size: 28px;
                background: transparent;
            }
        """)

    def leaveEvent(self, event):
        """Dim when not hovering."""
        self.setStyleSheet("""
            QLabel {
                color: rgba(255, 255, 255, 100);
                font-size: 28px;
                background: transparent;
            }
        """)
