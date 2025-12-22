"""
iOS-style toggle switch widget.
"""

from PyQt6.QtCore import QEasingCurve, QPropertyAnimation, Qt, pyqtProperty, pyqtSignal
from PyQt6.QtGui import QBrush, QColor, QPainter
from PyQt6.QtWidgets import QWidget


class ToggleSwitch(QWidget):
    """A custom iOS-style toggle switch widget."""

    # Signal emitted when the toggle state changes
    toggled = pyqtSignal(bool)

    def __init__(self, checked: bool = False, parent=None):
        super().__init__(parent)
        self._checked = checked
        self._circle_position = 22 if checked else 2
        self.setFixedSize(44, 24)
        self.setCursor(Qt.CursorShape.PointingHandCursor)

        # Animation for smooth toggle
        self._animation = QPropertyAnimation(self, b"circle_position", self)
        self._animation.setEasingCurve(QEasingCurve.Type.InOutCubic)
        self._animation.setDuration(150)

        # Legacy callback support (deprecated, use toggled signal instead)
        self._on_change_callback = None

    def get_circle_position(self):
        return self._circle_position

    def set_circle_position(self, pos):
        self._circle_position = pos
        self.update()

    circle_position = pyqtProperty(int, get_circle_position, set_circle_position)

    def isChecked(self) -> bool:
        return self._checked

    def setChecked(self, checked: bool):
        if self._checked != checked:
            self._checked = checked
            self._animation.setStartValue(self._circle_position)
            self._animation.setEndValue(22 if checked else 2)
            self._animation.start()

    def setOnChange(self, callback):
        """Set callback for when toggle state changes (deprecated, use toggled signal)."""
        self._on_change_callback = callback

    def mousePressEvent(self, event):
        self._checked = not self._checked
        self._animation.setStartValue(self._circle_position)
        self._animation.setEndValue(22 if self._checked else 2)
        self._animation.start()

        # Emit signal
        self.toggled.emit(self._checked)

        # Legacy callback support
        if self._on_change_callback:
            self._on_change_callback(self._checked)

    def paintEvent(self, event):
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)

        # Background
        if self._checked:
            bg_color = QColor(52, 199, 89)  # Green
        else:
            bg_color = QColor(120, 120, 128)  # Gray

        painter.setBrush(QBrush(bg_color))
        painter.setPen(Qt.PenStyle.NoPen)
        painter.drawRoundedRect(0, 0, 44, 24, 12, 12)

        # Circle
        painter.setBrush(QBrush(QColor(255, 255, 255)))
        painter.drawEllipse(self._circle_position, 2, 20, 20)
