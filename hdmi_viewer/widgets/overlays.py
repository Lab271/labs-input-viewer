"""
Overlay widgets for the viewer.

Includes InputNameOverlay, InfoPanel, and ScreenSaver components.
"""

import os

import cv2
import numpy as np
from PIL import Image
from PyQt6.QtCore import Qt, QTimer, pyqtSignal
from PyQt6.QtGui import QImage, QPixmap
from PyQt6.QtWidgets import QFrame, QLabel, QVBoxLayout

from hdmi_viewer.config import LOGO_FILENAME
from hdmi_viewer.utils import get_resource_path
from hdmi_viewer.widgets.base import HoverIcon


class InputNameOverlay(QLabel):
    """Overlay label to show the input name when switching."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.setStyleSheet(
            """
            QLabel {
                color: white;
                font-size: 48px;
                font-weight: bold;
                font-family: 'TT Interphases', 'Helvetica Neue', Arial, sans-serif;
                background-color: rgba(0, 0, 0, 180);
                border-radius: 16px;
                padding: 20px 40px;
            }
            """
        )
        self.hide()

        # Timer for auto-hide
        self._hide_timer = QTimer(self)
        self._hide_timer.timeout.connect(self.hide)
        self._hide_timer.setSingleShot(True)

    def show_name(self, name: str, duration_ms: int = 1500):
        """Show the input name overlay briefly."""
        self.setText(name)
        self.adjustSize()

        # Center in parent
        if self.parent():
            parent = self.parent()
            x = (parent.width() - self.width()) // 2
            y = (parent.height() - self.height()) // 2
            self.move(x, y)

        self.show()
        self.raise_()
        self._hide_timer.start(duration_ms)


class InfoPanel(QFrame):
    """Info overlay with keyboard shortcuts that shows on hover."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setStyleSheet("""
            QFrame {
                background-color: rgba(0, 0, 0, 200);
                border-radius: 8px;
            }
            QLabel {
                color: white;
                font-family: Arial, Helvetica;
                background: transparent;
            }
        """)

        layout = QVBoxLayout(self)
        layout.setContentsMargins(12, 10, 12, 10)
        layout.setSpacing(4)

        # Title
        title = QLabel("ⓘ Shortcuts")
        title.setStyleSheet("font-size: 14px; font-weight: bold; color: #88ccff;")
        layout.addWidget(title)

        # Shortcuts
        shortcuts = [
            ("D", "Dual view"),
            ("L", "Single left"),
            ("R", "Single right"),
            ("1-4", "Select input"),
            ("T", "Input thumbnails"),
            ("Space", "Freeze frame"),
            ("A", "Auto-switch"),
            ("F11", "Fullscreen"),
            ("Q", "Quit"),
        ]

        for key, desc in shortcuts:
            line = QLabel(f"<b>{key}</b>  {desc}")
            line.setStyleSheet("font-size: 12px;")
            layout.addWidget(line)

        # Attribution
        layout.addSpacing(8)
        attribution = QLabel("By Labs for _Space")
        attribution.setStyleSheet("font-size: 10px; color: rgba(255, 255, 255, 60);")
        layout.addWidget(attribution)

        self.adjustSize()
        self.hide()


class InfoIcon(HoverIcon):
    """Info icon that shows the InfoPanel on hover."""

    hover_changed = pyqtSignal(bool)

    def __init__(self, parent=None):
        super().__init__("ⓘ", parent)
        self.setFixedSize(35, 40)

    def enterEvent(self, event):
        super().enterEvent(event)
        self.hover_changed.emit(True)

    def leaveEvent(self, event):
        super().leaveEvent(event)
        self.hover_changed.emit(False)

    def mousePressEvent(self, event):
        pass  # Override to prevent click behavior


# DVD bounce colors (vibrant)
_BOUNCE_COLORS = [
    (0, 136, 255), (255, 0, 128), (0, 255, 128), (255, 200, 0),
    (255, 80, 0), (128, 0, 255), (0, 255, 255),
]


class ScreenSaver(QLabel):
    """Full-screen screensaver with DVD-style bouncing logo."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setStyleSheet("background-color: black;")
        self.setMouseTracking(True)  # Enable mouse tracking for cursor shake detection
        self.hide()
        self._state = None
        self._last_time = None

    def _init_state(self, width: int, height: int):
        """Initialize or reinitialize animation state for given dimensions."""
        import time
        logo = self._load_logo(width, height)
        blue_mask = self._create_blue_mask(logo) if logo else None

        self._state = {
            'size': (width, height),
            'logo': logo,
            'x': float(width // 4),
            'y': float(height // 4),
            'vx': 60.0,
            'vy': 40.0,
            'color_idx': 0,
            'blue_mask': blue_mask,
        }
        self._last_time = time.time()

    def _load_logo(self, width: int, height: int):
        """Load and scale logo for screensaver."""
        logo_path = get_resource_path(LOGO_FILENAME)
        if not os.path.exists(logo_path):
            return None
        logo = Image.open(logo_path).convert("RGBA")
        scale = min(width / logo.width, height / logo.height) * 0.15
        return logo.resize((int(logo.width * scale), int(logo.height * scale)), Image.Resampling.LANCZOS)

    def _create_blue_mask(self, logo):
        """Create mask for blue pixels in logo (for color cycling)."""
        arr = np.array(logo)
        r, b = arr[:, :, 0], arr[:, :, 2]
        alpha = arr[:, :, 3] if arr.shape[2] == 4 else np.full_like(r, 255)
        return (b > 150) & (b > r + 50) & (alpha > 100)

    def _update_position(self, dt: float, width: int, height: int) -> bool:
        """Update logo position and return True if bounced."""
        s = self._state
        logo_w, logo_h = s['logo'].size
        s['x'] += s['vx'] * dt
        s['y'] += s['vy'] * dt

        bounced = False
        if s['x'] <= 0 or s['x'] >= width - logo_w:
            s['x'] = max(0, min(s['x'], width - logo_w))
            s['vx'] = -s['vx']
            bounced = True
        if s['y'] <= 0 or s['y'] >= height - logo_h:
            s['y'] = max(0, min(s['y'], height - logo_h))
            s['vy'] = -s['vy']
            bounced = True
        return bounced

    def _render_logo(self, frame: np.ndarray):
        """Render colored logo onto frame."""
        s = self._state
        logo_arr = np.array(s['logo'])
        color = _BOUNCE_COLORS[s['color_idx']]

        if s['blue_mask'] is not None:
            logo_arr[:, :, 0][s['blue_mask']] = color[0]
            logo_arr[:, :, 1][s['blue_mask']] = color[1]
            logo_arr[:, :, 2][s['blue_mask']] = color[2]

        x, y = int(s['x']), int(s['y'])
        logo_h, logo_w = logo_arr.shape[:2]

        if logo_arr.shape[2] == 4:
            alpha = logo_arr[:, :, 3:4] / 255.0
            roi = frame[y:y + logo_h, x:x + logo_w]
            frame[y:y + logo_h, x:x + logo_w] = (alpha * logo_arr[:, :, :3] + (1 - alpha) * roi).astype(np.uint8)
        else:
            frame[y:y + logo_h, x:x + logo_w] = logo_arr[:, :, :3]

    def create_frame(self, width: int, height: int) -> np.ndarray:
        """Generate a DVD-style bouncing logo screensaver frame."""
        import time
        current_time = time.time()

        if not self._state or self._state.get('size') != (width, height):
            self._init_state(width, height)

        dt = min(current_time - (self._last_time or current_time), 0.1)
        self._last_time = current_time

        frame = np.zeros((height, width, 3), dtype=np.uint8)

        if self._state['logo']:
            if self._update_position(dt, width, height):
                self._state['color_idx'] = (self._state['color_idx'] + 1) % len(_BOUNCE_COLORS)
            self._render_logo(frame)

        return frame

    def show_frame(self, width: int, height: int):
        """Generate and display the screensaver frame."""
        self.setGeometry(0, 0, width, height)
        self.raise_()
        self.show()

        if width > 0 and height > 0:
            frame = self.create_frame(width, height)
            frame_rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            h, w, ch = frame_rgb.shape
            qimg = QImage(frame_rgb.data, w, h, ch * w, QImage.Format.Format_RGB888)
            self.setPixmap(QPixmap.fromImage(qimg))
