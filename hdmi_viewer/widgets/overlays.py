"""
Overlay widgets for the viewer.

Includes InputNameOverlay, InfoPanel, and ScreenSaver components.
"""

import os

import cv2
import numpy as np
from PIL import Image
from PyQt6.QtCore import Qt, QTimer, pyqtSignal
from PyQt6.QtGui import QEnterEvent, QImage, QPixmap
from PyQt6.QtWidgets import QFrame, QLabel, QVBoxLayout

from hdmi_viewer.config import LOGO_FILENAME
from hdmi_viewer.utils import get_resource_path


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


class InfoIcon(QFrame):
    """Small info icon that shows the InfoPanel on hover."""

    # Signal emitted when hover starts/ends
    hover_changed = pyqtSignal(bool)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setFixedSize(35, 40)
        self.setStyleSheet("background: transparent;")

        # Small info icon
        self.icon_label = QLabel("ⓘ", self)
        self.icon_label.setStyleSheet("""
            QLabel {
                color: rgba(255, 255, 255, 100);
                font-size: 28px;
                background: transparent;
            }
        """)
        self.icon_label.adjustSize()
        self.icon_label.move(5, 5)

    def enterEvent(self, event: QEnterEvent):
        """Show info panel when mouse enters."""
        self.icon_label.setStyleSheet("""
            QLabel {
                color: rgba(255, 255, 255, 255);
                font-size: 28px;
                background: transparent;
            }
        """)
        self.hover_changed.emit(True)

    def leaveEvent(self, event):
        """Hide info panel when mouse leaves."""
        self.icon_label.setStyleSheet("""
            QLabel {
                color: rgba(255, 255, 255, 100);
                font-size: 28px;
                background: transparent;
            }
        """)
        self.hover_changed.emit(False)


class ScreenSaver(QLabel):
    """Full-screen screensaver with DVD-style bouncing logo."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setStyleSheet("background-color: black;")
        self.hide()

        # Animation state
        self._ss = None

    def create_frame(self, width: int, height: int) -> np.ndarray:
        """Generate a DVD-style bouncing logo screensaver frame."""
        # Initialize screensaver state on first call or resize
        if not self._ss or self._ss.get('size') != (width, height):
            logo_path = get_resource_path(LOGO_FILENAME)
            logo = None
            if os.path.exists(logo_path):
                logo = Image.open(logo_path).convert("RGBA")
                # Scale logo to ~15% of screen
                scale = min(width / logo.width, height / logo.height) * 0.15
                new_w = int(logo.width * scale)
                new_h = int(logo.height * scale)
                logo = logo.resize((new_w, new_h), Image.Resampling.LANCZOS)

            # DVD bounce colors for the backslash (vibrant colors)
            colors = [
                (0, 136, 255),    # Blue (original)
                (255, 0, 128),    # Magenta
                (0, 255, 128),    # Green
                (255, 200, 0),    # Yellow
                (255, 80, 0),     # Orange
                (128, 0, 255),    # Purple
                (0, 255, 255),    # Cyan
            ]

            self._ss = {
                'size': (width, height),
                'logo': logo,
                'x': width // 4,
                'y': height // 4,
                'vx': 3,  # Velocity X (pixels per frame)
                'vy': 2,  # Velocity Y (pixels per frame)
                'colors': colors,
                'color_idx': 0,
                'blue_mask': None,
            }

            # Create mask for the blue backslash in the logo
            if logo:
                logo_arr = np.array(logo)
                # Find blue-ish pixels (the backslash is blue ~#0088ff)
                r, b = logo_arr[:, :, 0], logo_arr[:, :, 2]
                alpha = logo_arr[:, :, 3] if logo_arr.shape[2] == 4 else np.ones_like(r) * 255
                # Blue backslash: low red, medium green, high blue
                blue_mask = (b > 150) & (b > r + 50) & (alpha > 100)
                self._ss['blue_mask'] = blue_mask

        ss = self._ss

        # Create black frame
        frame = np.zeros((height, width, 3), dtype=np.uint8)

        if ss['logo']:
            logo_w, logo_h = ss['logo'].size

            # Update position
            ss['x'] += ss['vx']
            ss['y'] += ss['vy']

            # Bounce off walls and change color
            bounced = False
            if ss['x'] <= 0:
                ss['x'] = 0
                ss['vx'] = abs(ss['vx'])
                bounced = True
            elif ss['x'] >= width - logo_w:
                ss['x'] = width - logo_w
                ss['vx'] = -abs(ss['vx'])
                bounced = True

            if ss['y'] <= 0:
                ss['y'] = 0
                ss['vy'] = abs(ss['vy'])
                bounced = True
            elif ss['y'] >= height - logo_h:
                ss['y'] = height - logo_h
                ss['vy'] = -abs(ss['vy'])
                bounced = True

            # Change backslash color on bounce
            if bounced:
                ss['color_idx'] = (ss['color_idx'] + 1) % len(ss['colors'])

            # Create colored version of logo
            logo_arr = np.array(ss['logo'].copy())
            new_color = ss['colors'][ss['color_idx']]

            # Recolor the blue backslash
            if ss['blue_mask'] is not None:
                logo_arr[:, :, 0][ss['blue_mask']] = new_color[0]  # R
                logo_arr[:, :, 1][ss['blue_mask']] = new_color[1]  # G
                logo_arr[:, :, 2][ss['blue_mask']] = new_color[2]  # B

            # Alpha blend logo onto frame
            x, y = int(ss['x']), int(ss['y'])
            if logo_arr.shape[2] == 4:
                alpha = logo_arr[:, :, 3:4] / 255.0
                rgb = logo_arr[:, :, :3]
                roi = frame[y:y + logo_h, x:x + logo_w]
                blended = (alpha * rgb + (1 - alpha) * roi).astype(np.uint8)
                frame[y:y + logo_h, x:x + logo_w] = blended
            else:
                frame[y:y + logo_h, x:x + logo_w] = logo_arr[:, :, :3]

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
