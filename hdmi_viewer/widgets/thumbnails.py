"""
Thumbnails panel for quick input selection.
"""

from PyQt6.QtCore import Qt, pyqtSignal
from PyQt6.QtWidgets import (
    QFrame,
    QVBoxLayout,
    QGridLayout,
    QLabel,
)

from hdmi_viewer.config import get_all_input_configs


class ThumbnailsPanel(QFrame):
    """Panel showing thumbnails of all inputs for quick selection."""

    # Signal emitted when an input is selected
    input_selected = pyqtSignal(int)  # input index

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setFixedSize(320, 200)
        self.setStyleSheet("""
            QFrame {
                background-color: rgba(30, 30, 30, 230);
                border-radius: 12px;
                border: 1px solid rgba(255, 255, 255, 30);
            }
        """)

        self.thumbnail_labels = {}
        self._setup_ui()
        self.hide()

    def _setup_ui(self):
        """Set up the panel UI."""
        layout = QVBoxLayout(self)
        layout.setContentsMargins(10, 10, 10, 10)
        layout.setSpacing(8)

        # Title
        title = QLabel("⊞ Inputs")
        title.setStyleSheet(
            "color: rgba(255, 255, 255, 200); font-size: 14px; "
            "font-weight: bold; background: transparent;"
        )
        layout.addWidget(title)

        # Grid for thumbnails
        grid = QGridLayout()
        grid.setSpacing(6)

        for i in range(4):
            thumb_frame = QFrame()
            thumb_frame.setFixedSize(140, 80)
            thumb_frame.setStyleSheet("""
                QFrame {
                    background-color: rgba(0, 0, 0, 200);
                    border-radius: 6px;
                    border: 2px solid rgba(255, 255, 255, 30);
                }
                QFrame:hover {
                    border: 2px solid rgba(88, 166, 255, 200);
                }
            """)
            thumb_frame.setCursor(Qt.CursorShape.PointingHandCursor)
            thumb_frame.mousePressEvent = lambda e, idx=i: self._on_click(idx)

            thumb_layout = QVBoxLayout(thumb_frame)
            thumb_layout.setContentsMargins(4, 4, 4, 4)

            # Thumbnail image placeholder
            thumb_label = QLabel()
            thumb_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
            thumb_label.setStyleSheet("background: transparent;")
            thumb_layout.addWidget(thumb_label)

            # Input name
            name_label = QLabel(f"Input {i + 1}")
            name_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
            name_label.setStyleSheet(
                "color: white; font-size: 10px; background: transparent;"
            )
            thumb_layout.addWidget(name_label)

            self.thumbnail_labels[i] = {
                'frame': thumb_frame,
                'thumb': thumb_label,
                'name': name_label
            }
            grid.addWidget(thumb_frame, i // 2, i % 2)

        layout.addLayout(grid)

    def _on_click(self, index: int):
        """Handle click on a thumbnail."""
        self.input_selected.emit(index)
        self.hide()

    def refresh(self):
        """Update thumbnail labels with current input configurations."""
        input_configs = get_all_input_configs()

        for i, config in enumerate(input_configs):
            if i in self.thumbnail_labels:
                # Update name
                self.thumbnail_labels[i]['name'].setText(config.name)

                # Update style based on enabled state
                if config.enabled:
                    self.thumbnail_labels[i]['frame'].setStyleSheet("""
                        QFrame {
                            background-color: rgba(0, 0, 0, 200);
                            border-radius: 6px;
                            border: 2px solid rgba(52, 199, 89, 150);
                        }
                        QFrame:hover {
                            border: 2px solid rgba(88, 166, 255, 200);
                        }
                    """)
                else:
                    self.thumbnail_labels[i]['frame'].setStyleSheet("""
                        QFrame {
                            background-color: rgba(0, 0, 0, 200);
                            border-radius: 6px;
                            border: 2px solid rgba(255, 255, 255, 30);
                        }
                    """)

    def show_centered(self, parent_width: int, parent_height: int):
        """Show the panel centered in the parent."""
        x = (parent_width - self.width()) // 2
        y = (parent_height - self.height()) // 2
        self.move(x, y)
        self.refresh()
        self.show()
        self.raise_()
