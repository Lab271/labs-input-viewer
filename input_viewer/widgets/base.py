"""Base widget classes for reusable UI components."""

from PyQt6.QtCore import Qt, pyqtSignal
from PyQt6.QtWidgets import QLabel

# =============================================================================
# Shared Color Constants
# =============================================================================
COLOR_BG_PANEL = "rgba(30, 30, 30, 240)"
COLOR_BORDER = "rgba(255, 255, 255, 30)"
COLOR_HOVER = "rgba(80, 80, 80, 200)"
COLOR_ACCENT = "#58a6ff"
COLOR_DANGER = "rgba(255, 59, 48, 200)"


# =============================================================================
# Common Stylesheet Snippets
# =============================================================================
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

PANEL_STYLE = f"""
    QFrame {{
        background-color: {COLOR_BG_PANEL};
        border-radius: 12px;
        border: 1px solid {COLOR_BORDER};
    }}
    QLabel {{
        color: white;
        font-family: Arial, sans-serif;
        background: transparent;
    }}
"""

BUTTON_STYLE = f"""
    QPushButton {{
        background-color: rgba(60, 60, 60, 200);
        color: white;
        border: 1px solid {COLOR_BORDER};
        border-radius: 6px;
        padding: 6px 12px;
        font-size: 11px;
    }}
    QPushButton:hover {{
        background-color: {COLOR_HOVER};
    }}
    QPushButton:checked {{
        background-color: {COLOR_DANGER};
    }}
"""

SLIDER_STYLE = f"""
    QSlider::groove:horizontal {{
        height: 6px;
        background: {COLOR_BORDER};
        border-radius: 3px;
    }}
    QSlider::handle:horizontal {{
        width: 16px;
        height: 16px;
        margin: -5px 0;
        background: white;
        border-radius: 8px;
    }}
    QSlider::sub-page:horizontal {{
        background: {COLOR_ACCENT};
        border-radius: 3px;
    }}
"""


# =============================================================================
# Base Widgets
# =============================================================================
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
