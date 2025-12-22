"""
PyQt6 control panel UI for the mock server.
"""

import sys
import threading

from PyQt6.QtCore import QObject, Qt, QTimer, pyqtSignal
from PyQt6.QtGui import QImage, QPixmap
from PyQt6.QtWidgets import (
    QApplication,
    QCheckBox,
    QComboBox,
    QGroupBox,
    QHBoxLayout,
    QLabel,
    QMainWindow,
    QPushButton,
    QVBoxLayout,
    QWidget,
)

from .config import PATTERN_MAP, PatternType
from .server import VirtualCameraServer


class ServerThread(QObject):
    """Runs the virtual camera server in a background thread."""

    status_changed = pyqtSignal(str)
    error_occurred = pyqtSignal(str)

    def __init__(self, server: VirtualCameraServer):
        super().__init__()
        self.server = server
        self._thread: threading.Thread | None = None

    def start(self):
        """Start the server in a background thread."""
        if self._thread and self._thread.is_alive():
            return

        self._thread = threading.Thread(target=self._run, daemon=True)
        self._thread.start()
        self.status_changed.emit("Running")

    def stop(self):
        """Stop the server."""
        self.server.stop()
        if self._thread:
            self._thread.join(timeout=2.0)
        self.status_changed.emit("Stopped")

    def _run(self):
        """Thread target - runs the server."""
        try:
            self.server.start()
        except Exception as e:
            self.error_occurred.emit(str(e))
            self.status_changed.emit("Error")


class ControlPanel(QMainWindow):
    """
    Control panel window for the mock server.
    
    Allows changing patterns, toggling no-signal mode,
    and shows a preview of the output.
    """

    def __init__(self, server: VirtualCameraServer):
        super().__init__()
        self.server = server
        self.server_thread = ServerThread(server)

        self.setWindowTitle("HDMI Viewer Mock - Control Panel")
        self.setMinimumSize(500, 400)

        self._setup_ui()
        self._setup_connections()
        self._setup_preview_timer()

    def _setup_ui(self):
        """Create the UI layout."""
        central = QWidget()
        self.setCentralWidget(central)

        layout = QVBoxLayout(central)
        layout.setSpacing(15)

        # Status bar
        self.status_label = QLabel("Status: Stopped")
        self.status_label.setStyleSheet(
            "font-weight: bold; padding: 8px; "
            "background-color: #333; border-radius: 4px;"
        )
        layout.addWidget(self.status_label)

        # Preview
        preview_group = QGroupBox("Preview")
        preview_layout = QVBoxLayout(preview_group)

        self.preview_label = QLabel()
        self.preview_label.setMinimumSize(320, 180)
        self.preview_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.preview_label.setStyleSheet(
            "background-color: #1a1a1a; border: 1px solid #444;"
        )
        preview_layout.addWidget(self.preview_label)

        layout.addWidget(preview_group)

        # Pattern selection
        pattern_group = QGroupBox("Pattern Settings")
        pattern_layout = QVBoxLayout(pattern_group)

        # Pattern dropdown
        pattern_row = QHBoxLayout()
        pattern_row.addWidget(QLabel("Pattern:"))

        self.pattern_combo = QComboBox()
        for name in PATTERN_MAP.keys():
            self.pattern_combo.addItem(name.capitalize(), name)
        pattern_row.addWidget(self.pattern_combo)
        pattern_layout.addLayout(pattern_row)

        # No-signal toggle
        self.no_signal_check = QCheckBox("No Signal Mode")
        self.no_signal_check.setChecked(self.server.no_signal)
        pattern_layout.addWidget(self.no_signal_check)

        layout.addWidget(pattern_group)

        # Resolution info
        info_group = QGroupBox("Output Info")
        info_layout = QVBoxLayout(info_group)

        info_text = (
            f"Resolution: {self.server.width}x{self.server.height}\n"
            f"Frame Rate: {self.server.fps} fps"
        )
        info_label = QLabel(info_text)
        info_label.setStyleSheet("color: #888;")
        info_layout.addWidget(info_label)

        layout.addWidget(info_group)

        # Control buttons
        button_layout = QHBoxLayout()

        self.start_button = QPushButton("Start Server")
        self.start_button.setStyleSheet(
            "QPushButton { background-color: #2d5a27; padding: 10px; }"
            "QPushButton:hover { background-color: #3d7a37; }"
        )
        button_layout.addWidget(self.start_button)

        self.stop_button = QPushButton("Stop Server")
        self.stop_button.setEnabled(False)
        self.stop_button.setStyleSheet(
            "QPushButton { background-color: #5a2727; padding: 10px; }"
            "QPushButton:hover { background-color: #7a3737; }"
        )
        button_layout.addWidget(self.stop_button)

        layout.addLayout(button_layout)

        # Spacer
        layout.addStretch()

    def _setup_connections(self):
        """Connect signals and slots."""
        self.start_button.clicked.connect(self._on_start)
        self.stop_button.clicked.connect(self._on_stop)

        self.pattern_combo.currentTextChanged.connect(self._on_pattern_changed)
        self.no_signal_check.toggled.connect(self._on_no_signal_toggled)

        self.server_thread.status_changed.connect(self._on_status_changed)
        self.server_thread.error_occurred.connect(self._on_error)

    def _setup_preview_timer(self):
        """Setup timer for preview updates."""
        self.preview_timer = QTimer()
        self.preview_timer.timeout.connect(self._update_preview)
        self.preview_timer.start(100)  # 10 fps preview

    def _on_start(self):
        """Handle start button click."""
        self.start_button.setEnabled(False)
        self.stop_button.setEnabled(True)
        self.server_thread.start()

    def _on_stop(self):
        """Handle stop button click."""
        self.server_thread.stop()
        self.start_button.setEnabled(True)
        self.stop_button.setEnabled(False)

    def _on_pattern_changed(self, text: str):
        """Handle pattern selection change."""
        pattern_name = text.lower()
        pattern = PATTERN_MAP.get(pattern_name, PatternType.BARS)
        self.server.set_pattern(pattern)

    def _on_no_signal_toggled(self, checked: bool):
        """Handle no-signal checkbox toggle."""
        self.server.set_no_signal(checked)

    def _on_status_changed(self, status: str):
        """Handle server status change."""
        color = "#2d5a27" if status == "Running" else "#5a2727"
        self.status_label.setText(f"Status: {status}")
        self.status_label.setStyleSheet(
            f"font-weight: bold; padding: 8px; "
            f"background-color: {color}; border-radius: 4px;"
        )

    def _on_error(self, error: str):
        """Handle server error."""
        self.status_label.setText(f"Error: {error}")
        self.status_label.setStyleSheet(
            "font-weight: bold; padding: 8px; "
            "background-color: #5a2727; border-radius: 4px;"
        )
        self.start_button.setEnabled(True)
        self.stop_button.setEnabled(False)

    def _update_preview(self):
        """Update the preview image."""
        frame = self.server.run_single_frame()
        if frame is None:
            return

        # Scale down for preview
        preview_w = 320
        preview_h = 180

        import cv2
        frame_small = cv2.resize(frame, (preview_w, preview_h))
        frame_rgb = cv2.cvtColor(frame_small, cv2.COLOR_BGR2RGB)

        # Convert to QPixmap
        h, w, ch = frame_rgb.shape
        bytes_per_line = ch * w

        qimg = QImage(
            frame_rgb.data,
            w, h,
            bytes_per_line,
            QImage.Format.Format_RGB888
        )
        pixmap = QPixmap.fromImage(qimg)

        self.preview_label.setPixmap(pixmap)

    def closeEvent(self, event):
        """Handle window close."""
        self.server_thread.stop()
        self.preview_timer.stop()
        event.accept()


def run_with_gui(server: VirtualCameraServer):
    """Run the mock server with a GUI control panel."""
    app = QApplication(sys.argv)

    # Dark theme
    app.setStyle("Fusion")

    from PyQt6.QtGui import QColor, QPalette

    palette = QPalette()
    palette.setColor(QPalette.ColorRole.Window, QColor(53, 53, 53))
    palette.setColor(QPalette.ColorRole.WindowText, QColor(255, 255, 255))
    palette.setColor(QPalette.ColorRole.Base, QColor(25, 25, 25))
    palette.setColor(QPalette.ColorRole.AlternateBase, QColor(53, 53, 53))
    palette.setColor(QPalette.ColorRole.ToolTipBase, QColor(255, 255, 255))
    palette.setColor(QPalette.ColorRole.ToolTipText, QColor(255, 255, 255))
    palette.setColor(QPalette.ColorRole.Text, QColor(255, 255, 255))
    palette.setColor(QPalette.ColorRole.Button, QColor(53, 53, 53))
    palette.setColor(QPalette.ColorRole.ButtonText, QColor(255, 255, 255))
    palette.setColor(QPalette.ColorRole.BrightText, QColor(255, 0, 0))
    palette.setColor(QPalette.ColorRole.Link, QColor(42, 130, 218))
    palette.setColor(QPalette.ColorRole.Highlight, QColor(42, 130, 218))
    palette.setColor(QPalette.ColorRole.HighlightedText, QColor(0, 0, 0))

    app.setPalette(palette)

    window = ControlPanel(server)
    window.show()

    sys.exit(app.exec())
