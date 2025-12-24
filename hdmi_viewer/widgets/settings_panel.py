"""
Settings panel widget for configuring inputs.
"""

from PyQt6.QtCore import Qt, pyqtSignal
from PyQt6.QtWidgets import (
    QFrame,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QPushButton,
    QScrollArea,
    QVBoxLayout,
    QWidget,
)

from hdmi_viewer.config import (
    load_settings,
    reload_config,
    save_settings,
)
from hdmi_viewer.widgets.toggle_switch import ToggleSwitch


class SettingsPanel(QFrame):
    """Settings panel for configuring input sources."""

    # Signals
    name_changed = pyqtSignal(int, str)  # (index, new_name)
    enabled_changed = pyqtSignal(int, bool)  # (index, enabled)
    default_changed = pyqtSignal(int)  # (index)
    settings_closed = pyqtSignal()
    config_reloaded = pyqtSignal()  # Emitted when config is reloaded

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setFixedSize(400, 450)
        self.setStyleSheet("""
            QFrame#settingsPanel {
                background-color: rgba(30, 30, 30, 240);
                border-radius: 12px;
                border: 1px solid rgba(255, 255, 255, 30);
            }
            QLabel {
                color: white;
                font-family: 'TT Interphases', Arial, sans-serif;
                background: transparent;
            }
            QLineEdit {
                background-color: rgba(60, 60, 60, 200);
                border: 1px solid rgba(255, 255, 255, 30);
                border-radius: 6px;
                color: white;
                padding: 6px 10px;
                font-size: 14px;
            }
            QLineEdit:focus {
                border: 1px solid rgba(88, 166, 255, 200);
            }
        """)
        self.setObjectName("settingsPanel")

        self.input_widgets = []
        self._setup_ui()
        self.hide()

    def _setup_ui(self):
        """Set up the panel UI."""
        layout = QVBoxLayout(self)
        layout.setContentsMargins(20, 20, 20, 20)
        layout.setSpacing(16)

        # Header with title and close button
        header = QHBoxLayout()
        title = QLabel("⚙ Input Settings")
        title.setStyleSheet("font-size: 18px; font-weight: bold; color: #88ccff;")
        header.addWidget(title)
        header.addStretch()

        close_btn = QPushButton("✕")
        close_btn.setFixedSize(30, 30)
        close_btn.setStyleSheet("""
            QPushButton {
                background-color: transparent;
                color: rgba(255, 255, 255, 150);
                font-size: 18px;
                border: none;
                border-radius: 15px;
            }
            QPushButton:hover {
                background-color: rgba(255, 255, 255, 30);
                color: white;
            }
        """)
        close_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        close_btn.clicked.connect(self._on_close)
        header.addWidget(close_btn)
        layout.addLayout(header)

        # Scroll area for inputs
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        scroll.setStyleSheet("""
            QScrollArea {
                background: transparent;
                border: none;
            }
            QWidget#scrollContent {
                background: transparent;
            }
        """)

        scroll_content = QWidget()
        scroll_content.setObjectName("scrollContent")
        self.inputs_layout = QVBoxLayout(scroll_content)
        self.inputs_layout.setSpacing(12)
        self.inputs_layout.setContentsMargins(0, 0, 0, 0)

        scroll.setWidget(scroll_content)
        layout.addWidget(scroll)

    def _on_close(self):
        """Handle close button click."""
        self.hide()
        self.settings_closed.emit()

    def refresh(self):
        """Refresh the settings panel with current input configurations."""
        # Clear existing widgets
        while self.inputs_layout.count():
            child = self.inputs_layout.takeAt(0)
            if child.widget():
                child.widget().deleteLater()

        self.input_widgets = []

        # Load current settings
        settings = load_settings()
        inputs = settings.get("inputs", [])

        for i, input_data in enumerate(inputs):
            input_frame = QFrame()
            input_frame.setStyleSheet("""
                QFrame {
                    background-color: rgba(50, 50, 50, 150);
                    border-radius: 8px;
                    padding: 8px;
                }
            """)

            input_layout = QVBoxLayout(input_frame)
            input_layout.setContentsMargins(12, 10, 12, 10)
            input_layout.setSpacing(8)

            # Input header with index
            header_label = QLabel(f"Input {input_data.get('index', i)}")
            header_label.setStyleSheet(
                "font-size: 12px; color: rgba(255, 255, 255, 100);"
            )
            input_layout.addWidget(header_label)

            # Name field
            name_row = QHBoxLayout()
            name_label = QLabel("Name:")
            name_label.setFixedWidth(60)
            name_label.setStyleSheet("font-size: 14px;")
            name_edit = QLineEdit(input_data.get("name", f"Input {i}"))
            name_edit.setProperty("input_index", i)
            name_edit.textChanged.connect(
                lambda text, idx=i: self._on_name_changed(idx, text)
            )
            name_row.addWidget(name_label)
            name_row.addWidget(name_edit)
            input_layout.addLayout(name_row)

            # Toggle row
            toggle_row = QHBoxLayout()

            # Enabled toggle
            enabled_label = QLabel("Enabled:")
            enabled_label.setStyleSheet("font-size: 14px;")
            enabled_toggle = ToggleSwitch(input_data.get("enabled", True))
            enabled_toggle.toggled.connect(
                lambda checked, idx=i: self._on_enabled_changed(idx, checked)
            )

            # Default toggle
            default_label = QLabel("Default:")
            default_label.setStyleSheet("font-size: 14px;")
            default_toggle = ToggleSwitch(input_data.get("default", False))
            default_toggle.toggled.connect(
                lambda checked, idx=i: self._on_default_changed(idx, checked)
            )

            toggle_row.addWidget(enabled_label)
            toggle_row.addWidget(enabled_toggle)
            toggle_row.addSpacing(20)
            toggle_row.addWidget(default_label)
            toggle_row.addWidget(default_toggle)
            toggle_row.addStretch()
            input_layout.addLayout(toggle_row)

            self.inputs_layout.addWidget(input_frame)
            self.input_widgets.append({
                "name_edit": name_edit,
                "enabled_toggle": enabled_toggle,
                "default_toggle": default_toggle,
            })

        self.inputs_layout.addStretch()

    def _on_name_changed(self, index: int, name: str):
        """Handle input name change."""
        settings = load_settings()
        if index < len(settings.get("inputs", [])):
            settings["inputs"][index]["name"] = name
            save_settings(settings)
            self.name_changed.emit(index, name)

    def _on_enabled_changed(self, index: int, enabled: bool):
        """Handle input enabled toggle change."""
        settings = load_settings()
        if index < len(settings.get("inputs", [])):
            settings["inputs"][index]["enabled"] = enabled

            # If disabling and this was default, clear default
            if not enabled and settings["inputs"][index].get("default", False):
                settings["inputs"][index]["default"] = False
                # Update toggle UI
                if index < len(self.input_widgets):
                    self.input_widgets[index]["default_toggle"].setChecked(False)

            # Ensure at least one enabled input is default
            self._ensure_default_exists(settings["inputs"])

            save_settings(settings)
            reload_config()
            self.enabled_changed.emit(index, enabled)
            self.config_reloaded.emit()

    def _on_default_changed(self, index: int, is_default: bool):
        """Handle input default toggle change - ensure only one is default."""
        settings = load_settings()
        inputs = settings.get("inputs", [])

        if is_default:
            # Clear default from all other inputs
            for i, inp in enumerate(inputs):
                if i != index:
                    inp["default"] = False
                    # Update toggle UI
                    if i < len(self.input_widgets):
                        self.input_widgets[i]["default_toggle"].setChecked(False)

            # Set this one as default (only if enabled)
            if inputs[index].get("enabled", True):
                inputs[index]["default"] = True
            else:
                # Can't set disabled input as default
                inputs[index]["default"] = False
                if index < len(self.input_widgets):
                    self.input_widgets[index]["default_toggle"].setChecked(False)
        else:
            inputs[index]["default"] = False

        # Ensure at least one enabled input is default
        self._ensure_default_exists(inputs)

        save_settings(settings)
        reload_config()
        self.default_changed.emit(index)
        self.config_reloaded.emit()

    def _ensure_default_exists(self, inputs: list[dict]):
        """Ensure at least one enabled input is set as default."""
        # Check if any enabled input is default
        has_default = any(
            inp.get("default", False) and inp.get("enabled", True) for inp in inputs
        )

        if not has_default:
            # Set first enabled input as default
            for i, inp in enumerate(inputs):
                if inp.get("enabled", True):
                    inp["default"] = True
                    # Update toggle UI if widgets exist
                    if i < len(self.input_widgets):
                        self.input_widgets[i]["default_toggle"].setChecked(True)
                    break
