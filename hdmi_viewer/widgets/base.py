"""
Base widget classes for reusable UI components.
"""

from PyQt6.QtCore import Qt, pyqtSignal
from PyQt6.QtWidgets import QLabel


# Common stylesheet values
ICON_STYLE_DIM = """
    QLabel {
        color: rgba(255, 255, 255, 100);
        font-size: 28px;
        background: transparent;
    }
"""

ICON_STYLE_BRIGHT = """
    QLabel {
        color: rgba(255, 255, 255, 255);
        font-size: 28px;
        background: transparent;
    }
"""

PANEL_STYLE = """
    QFrame {
        background-color: rgba(30, 30, 30, 240);
        border-radius: 12px;
        border: 1px solid rgba(255, 255, 255, 30);
    }
    QLabel {
        color: white;
        font-family: Arial, sans-serif;
        background: transparent;
    }
"""

BUTTON_STYLE = """
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
"""

SLIDER_STYLE = """
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
"""


class HoverIcon(QLabel):
    """Base class for clickable icons with hover effect."""

    clicked = pyqtSignal()

    def __init__(self, icon_char: str, parent=None):
        super().__init__(icon_char, parent)
        self.setStyleSheet(ICON_STYLE_DIM)
        self.adjustSize()
        self.setCursor(Qt.CursorShape.PointingHandCursor)

    def enterEvent(self, event):
        """Highlight on hover."""
        self.setStyleSheet(ICON_STYLE_BRIGHT)

    def leaveEvent(self, event):
        """Dim when not hovering."""
        self.setStyleSheet(ICON_STYLE_DIM)

    def mousePressEvent(self, event):
        """Emit clicked signal."""
        self.clicked.emit()
