#!/usr/bin/env python3
"""
Dual Elgato Capture Card Viewer for Ultrawide Displays

Displays two camera feeds side by side with configurable spacing.
Designed for 6000x1200 resolution displays.

Usage:
    python HDMI-viewer.py                  # Production mode (real cameras)
    python HDMI-viewer.py --mock           # Test mode (animated mock sources)
    python HDMI-viewer.py --switch-signals # Cycle signal/no-signal every 10s
    python HDMI-viewer.py --no-signal      # Always show no-signal overlay

Keyboard shortcuts:
    F11 / F  - Toggle fullscreen
    Escape   - Exit fullscreen (or quit if windowed)
    Q        - Quit

    Layout switching:
    D        - Dual view (both feeds side by side)
    1        - Single view: left feed centered
    2        - Single view: right feed centered

    Camera switching (test mode shows simulated feeds):
    Left/Right arrows - Change camera index for active single feed
    Shift+Left/Right  - Change left camera index (in dual mode)
    Ctrl+Left/Right   - Change right camera index (in dual mode)

Requirements:
    pip install PyQt6 opencv-python numpy Pillow
"""

import argparse
import os
import sys
from enum import Enum, auto

import cv2
import numpy as np
from PIL import Image, ImageDraw, ImageFont
from PyQt6.QtCore import QTimer, Qt
from PyQt6.QtGui import QEnterEvent, QImage, QPixmap
from PyQt6.QtWidgets import (
    QApplication,
    QFrame,
    QHBoxLayout,
    QLabel,
    QSizePolicy,
    QVBoxLayout,
    QWidget,
)


# =============================================================================
# LOGGING UTILITY
# =============================================================================


class Colors:
    """ANSI color codes for terminal output."""

    RESET = "\033[0m"
    BOLD = "\033[1m"
    DIM = "\033[2m"
    RED = "\033[91m"
    GREEN = "\033[92m"
    YELLOW = "\033[93m"
    BLUE = "\033[94m"
    MAGENTA = "\033[95m"
    CYAN = "\033[96m"
    WHITE = "\033[97m"


class Log:
    """Simple colored logging utility with verbose mode support."""

    _verbose = False

    @classmethod
    def set_verbose(cls, enabled: bool):
        """Enable or disable verbose logging."""
        cls._verbose = enabled

    @classmethod
    def _print(cls, color: str, prefix: str, msg: str, force: bool = False):
        """Print a colored message if verbose mode is enabled or force is True."""
        if cls._verbose or force:
            print(f"{color}{prefix}{Colors.RESET} {msg}")

    @classmethod
    def info(cls, msg: str, force: bool = False):
        """Print info message (cyan)."""
        cls._print(Colors.CYAN, "ℹ", msg, force)

    @classmethod
    def success(cls, msg: str, force: bool = False):
        """Print success message (green)."""
        cls._print(Colors.GREEN, "✓", msg, force)

    @classmethod
    def warning(cls, msg: str, force: bool = False):
        """Print warning message (yellow)."""
        cls._print(Colors.YELLOW, "⚠", msg, force)

    @classmethod
    def error(cls, msg: str, force: bool = False):
        """Print error message (red)."""
        cls._print(Colors.RED, "✗", msg, force)

    @classmethod
    def debug(cls, msg: str):
        """Print debug message (dim, only in verbose mode)."""
        if cls._verbose:
            print(f"{Colors.DIM}  {msg}{Colors.RESET}")

    @classmethod
    def header(cls, msg: str, force: bool = False):
        """Print a header message (bold magenta)."""
        if cls._verbose or force:
            print(f"{Colors.BOLD}{Colors.MAGENTA}{'=' * 60}{Colors.RESET}")
            print(f"{Colors.BOLD}{Colors.MAGENTA}{msg}{Colors.RESET}")
            print(f"{Colors.BOLD}{Colors.MAGENTA}{'=' * 60}{Colors.RESET}")


class LayoutMode(Enum):
    """Available layout modes for the viewer."""

    DUAL = auto()  # Both feeds side by side
    SINGLE_LEFT = auto()  # Only left feed, centered
    SINGLE_RIGHT = auto()  # Only right feed, centered


# =============================================================================
# CONFIGURATION - Adjust these to your setup
# =============================================================================

# Camera indices (from your Cam Link Pro)
# Cam Link Pro typically exposes 4 inputs as indices 0-3
LEFT_CAMERA_INDEX = 0  # Left display (default)
RIGHT_CAMERA_INDEX = 0  # Right display (default)
AVAILABLE_CAMERA_INDICES = [0, 1, 2, 3]  # All available camera inputs

# Capture settings
TARGET_WIDTH = 1920
TARGET_HEIGHT = 1080
TARGET_FPS = 30

# Layout spacing (in pixels)
SIDE_MARGIN = 150  # Black space on left and right edges
CENTER_GAP = 200  # Black space between the two feeds

# Logo settings
LOGO_FILENAME = "Logo-3-OnDark.png"  # Logo file in the same folder as this script

# No signal message
NO_SIGNAL_MESSAGE = "Please connect your computer to the HDMI"

# Display settings
FULLSCREEN = True
TIMER_INTERVAL_MS = 30


# =============================================================================
# APPLICATION
# =============================================================================


class CameraFeed:
    """Handles a single camera capture (real or mock)."""

    def __init__(
        self,
        camera_index: int,
        test_mode: bool = False,
        label: str = "FEED",
        switch_signals: bool = False,
        always_no_signal: bool = False,
    ):
        self.index = camera_index
        self.test_mode = test_mode
        self.switch_signals = switch_signals
        self.always_no_signal = always_no_signal
        self.last_frame = None
        self.static_frame_count = 0

        if test_mode:
            from mock_sources import create_mock_feed

            self.cap = create_mock_feed(
                camera_index,
                TARGET_WIDTH,
                TARGET_HEIGHT,
                TARGET_FPS,
                label,
                switch_signals=switch_signals,
                always_no_signal=always_no_signal,
            )
            if always_no_signal:
                mode_str = "NO-SIGNAL"
            elif switch_signals:
                mode_str = "SWITCH-SIGNALS"
            else:
                mode_str = "MOCK"
            Log.success(
                f"Mock camera {camera_index} ({label}) [{mode_str}]: {TARGET_WIDTH}x{TARGET_HEIGHT} @ {TARGET_FPS} FPS"
            )
        else:
            self.cap = self._open_camera(camera_index)
            if self.cap and self.cap.isOpened():
                w = int(self.cap.get(cv2.CAP_PROP_FRAME_WIDTH))
                h = int(self.cap.get(cv2.CAP_PROP_FRAME_HEIGHT))
                fps = self.cap.get(cv2.CAP_PROP_FPS)
                Log.success(f"Camera {camera_index}: {w}x{h} @ {fps:.1f} FPS")
            else:
                Log.error(f"Camera {camera_index}: FAILED TO OPEN")

    def _open_camera(self, index: int) -> cv2.VideoCapture:
        if sys.platform.startswith("win"):
            cap = cv2.VideoCapture(index, cv2.CAP_DSHOW)
        elif sys.platform == "darwin":
            cap = cv2.VideoCapture(index, cv2.CAP_AVFOUNDATION)
        else:
            cap = cv2.VideoCapture(index, cv2.CAP_V4L2)
            if not cap.isOpened():
                cap = cv2.VideoCapture(index)

        if cap.isOpened():
            cap.set(cv2.CAP_PROP_FRAME_WIDTH, TARGET_WIDTH)
            cap.set(cv2.CAP_PROP_FRAME_HEIGHT, TARGET_HEIGHT)
            cap.set(cv2.CAP_PROP_FPS, TARGET_FPS)
            cap.set(cv2.CAP_PROP_BUFFERSIZE, 1)

        return cap

    def _is_no_signal(self, frame) -> bool:
        """
        Detect if the frame is the Elgato 'No Signal' screen.
        The no signal screen is static, so we detect if frames aren't changing.
        Also checks for the characteristic dark gray color of the Elgato screen.
        """
        # Check if frame is mostly uniform dark gray (Elgato no signal screen)
        # The Elgato screen is typically a dark gray around RGB(40-60, 40-60, 40-60)
        gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
        mean_val = np.mean(gray)
        std_val = np.std(gray)

        # If the image has very low variation and is dark, likely no signal
        # Adjust these thresholds if needed for your specific Elgato screen
        if std_val < 30 and 20 < mean_val < 80:
            # Additionally check if frame is static (not changing)
            if self.last_frame is not None:
                diff = cv2.absdiff(
                    gray, cv2.cvtColor(self.last_frame, cv2.COLOR_BGR2GRAY)
                )
                if np.mean(diff) < 1:  # Almost no difference
                    self.static_frame_count += 1
                else:
                    self.static_frame_count = 0

            self.last_frame = frame.copy()

            # If static for multiple frames, it's likely the no signal screen
            if self.static_frame_count > 5:
                return True
        else:
            self.static_frame_count = 0
            self.last_frame = frame.copy()

        return False

    def read_frame(self):
        """Read a frame and convert to QPixmap. Returns (pixmap, has_signal)."""
        if not self.cap or not self.cap.isOpened():
            return None, False

        ret, frame = self.cap.read()
        if not ret or frame is None:
            return None, False

        # Check for no signal
        if self._is_no_signal(frame):
            return None, False

        frame_rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
        h, w, ch = frame_rgb.shape
        bytes_per_line = ch * w

        qimg = QImage(
            frame_rgb.copy().data, w, h, bytes_per_line, QImage.Format.Format_RGB888
        )

        return QPixmap.fromImage(qimg), True

    def release(self):
        if self.cap and self.cap.isOpened():
            self.cap.release()

    def toggle_signal(self) -> bool:
        """Toggle signal state (test mode only). Returns new signal state."""
        if self.test_mode and hasattr(self.cap, "toggle_signal"):
            return self.cap.toggle_signal()
        return True


class DualVideoViewer(QWidget):
    def __init__(
        self,
        test_mode: bool = False,
        switch_signals: bool = False,
        always_no_signal: bool = False,
    ):
        super().__init__()
        self.test_mode = test_mode
        self.switch_signals = switch_signals
        self.always_no_signal = always_no_signal
        self.layout_mode = LayoutMode.DUAL

        # Camera indices (can be changed at runtime)
        self.left_camera_index = LEFT_CAMERA_INDEX
        self.right_camera_index = RIGHT_CAMERA_INDEX

        # Open both cameras
        self.left_feed = CameraFeed(
            self.left_camera_index, test_mode, "LEFT", switch_signals, always_no_signal
        )
        self.right_feed = CameraFeed(
            self.right_camera_index,
            test_mode,
            "RIGHT",
            switch_signals,
            always_no_signal,
        )

        # Window setup
        self.setWindowTitle("Dual Elgato Viewer")
        self.setStyleSheet("background-color: black;")

        # Create video labels
        self.left_label = self._create_video_label()
        self.right_label = self._create_video_label()

        # Spacers and logo
        self.left_spacer = self._create_spacer(SIDE_MARGIN)
        self.center_logo = self._create_logo_label(CENTER_GAP)
        self.right_spacer = self._create_spacer(SIDE_MARGIN)

        # Layout: [margin] [video] [logo] [video] [margin]
        layout = QHBoxLayout()
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(0)

        layout.addWidget(self.left_spacer)
        layout.addWidget(self.left_label, 1)  # stretch factor 1
        layout.addWidget(self.center_logo)
        layout.addWidget(self.right_label, 1)  # stretch factor 1
        layout.addWidget(self.right_spacer)

        self.setLayout(layout)

        # Info overlay in bottom-left corner
        self.info_overlay = self._create_info_overlay()

        # Start fullscreen if configured
        if FULLSCREEN:
            self.showFullScreen()
        else:
            self.resize(1920, 600)

        # Frame update timer
        self.timer = QTimer(self)
        self.timer.timeout.connect(self.update_frames)
        self.timer.start(TIMER_INTERVAL_MS)

        # Animation state for no-signal pulse
        self.pulse_phase = 0

    def _create_video_label(self) -> QLabel:
        """Create a video display label."""
        label = QLabel()
        label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        label.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        label.setStyleSheet("background-color: black;")
        return label

    def _create_no_signal_frame(self, width: int, height: int) -> np.ndarray:
        """Generate no-signal frame using the custom image with animated HDMI cable."""
        # Pre-render all animation frames for smooth playback
        cache_key = (width, height)
        if (
            not hasattr(self, "_no_signal_cache")
            or self._no_signal_cache.get("size") != cache_key
        ):
            script_dir = os.path.dirname(os.path.abspath(__file__))
            icon_path = os.path.join(script_dir, "no_signal_icon.png")
            if os.path.exists(icon_path):
                icon = Image.open(icon_path).convert("RGBA")
                cable_split_x = 980

                # Calculate scale to fit the image in the frame
                icon_w, icon_h = icon.width, icon.height
                scale = min(width / icon_w, height / icon_h) * 0.6

                new_w = int(icon_w * scale)
                new_h = int(icon_h * scale)
                cable_w = int(cable_split_x * scale)

                # Pre-scale the parts once
                cable_part = icon.crop((0, 0, cable_split_x, icon.height))
                rest_part = icon.crop((cable_split_x, 0, icon.width, icon.height))

                cable_scaled = cable_part.resize(
                    (cable_w, new_h), Image.Resampling.LANCZOS
                )
                rest_scaled = rest_part.resize(
                    (new_w - cable_w, new_h), Image.Resampling.LANCZOS
                )

                base_x = (width - new_w) // 2
                base_y = (height - new_h) // 2

                # Pre-render ALL animation frames (120 frames for smooth loop)
                num_frames = 120
                max_offset = int(50 * scale)
                frames = []

                # Text settings - try TT Interphases, fallback to system fonts
                text = "No signal detected"
                text_font = None
                font_size = 32
                font_paths = [
                    "/Library/Fonts/TT Interphases Pro Trial Medium.ttf",
                    "/Library/Fonts/TT Interphases Pro Medium.ttf",
                    "/Library/Fonts/TTInterphasesPro-Medium.ttf",
                    "C:/Windows/Fonts/TT Interphases Pro Trial Medium.ttf",
                    "C:/Windows/Fonts/TTInterphasesPro-Medium.ttf",
                    "/System/Library/Fonts/Helvetica.ttc",
                    "C:/Windows/Fonts/segoeui.ttf",
                ]
                for path in font_paths:
                    try:
                        text_font = ImageFont.truetype(path, font_size)
                        break
                    except (OSError, IOError):
                        continue
                if text_font is None:
                    text_font = ImageFont.load_default()

                # Calculate text position
                temp_img = Image.new("RGB", (width, height))
                temp_draw = ImageDraw.Draw(temp_img)
                text_bbox = temp_draw.textbbox((0, 0), text, font=text_font)
                text_width = text_bbox[2] - text_bbox[0]
                text_x = (width - text_width) // 2
                text_y = base_y + new_h + 30  # Below the image

                for i in range(num_frames):
                    # Sine wave animation
                    t = i / num_frames * 2 * np.pi
                    ease = (np.sin(t) + 1) / 2
                    cable_offset = int(-max_offset * (1 - ease))

                    # Create frame
                    img = Image.new("RGB", (width, height), (0, 0, 0))
                    draw = ImageDraw.Draw(img)
                    rest_x = base_x + cable_w
                    img.paste(rest_scaled.convert("RGB"), (rest_x, base_y))
                    cable_x = base_x + cable_offset
                    img.paste(cable_scaled.convert("RGB"), (cable_x, base_y))

                    # Add text using PIL
                    draw.text(
                        (text_x, text_y), text, font=text_font, fill=(150, 150, 150)
                    )

                    # Convert to BGR numpy array
                    frame = cv2.cvtColor(np.array(img), cv2.COLOR_RGB2BGR)
                    frames.append(frame)

                self._no_signal_cache = {
                    "size": cache_key,
                    "frames": frames,
                    "num_frames": num_frames,
                }
                self._no_signal_frame_idx = 0
                Log.debug(
                    f"Pre-rendered {num_frames} animation frames for no-signal icon"
                )
            else:
                self._no_signal_cache = {"size": cache_key, "frames": None}
                Log.warning(f"No signal icon not found: {icon_path}")

        cache = self._no_signal_cache

        if cache.get("frames") is None:
            # Fallback: simple text
            frame = np.zeros((height, width, 3), dtype=np.uint8)
            cv2.putText(
                frame,
                "No Signal",
                (width // 2 - 100, height // 2),
                cv2.FONT_HERSHEY_SIMPLEX,
                1.5,
                (128, 128, 128),
                2,
            )
            return frame

        # Simply return the next pre-rendered frame
        frame = cache["frames"][self._no_signal_frame_idx]
        self._no_signal_frame_idx = (self._no_signal_frame_idx + 1) % cache[
            "num_frames"
        ]

        return frame

    def _create_spacer(self, width: int) -> QLabel:
        spacer = QLabel()
        spacer.setFixedWidth(width)
        spacer.setStyleSheet("background-color: black;")
        return spacer

    def _create_info_overlay(self) -> QFrame:
        """Create an info overlay with keyboard shortcuts that shows on hover."""
        # Container that handles hover events
        container = QFrame(self)
        container.setFixedSize(200, 200)  # Hover detection area
        container.setStyleSheet("background: transparent;")

        # Small info icon (always visible)
        self.info_icon = QLabel("ⓘ", container)
        self.info_icon.setStyleSheet("""
            QLabel {
                color: rgba(255, 255, 255, 100);
                font-size: 24px;
                background: transparent;
            }
        """)
        self.info_icon.adjustSize()
        self.info_icon.move(10, 160)

        # The actual info panel (hidden by default)
        self.info_panel = QFrame(container)
        self.info_panel.setStyleSheet("""
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

        layout = QVBoxLayout(self.info_panel)
        layout.setContentsMargins(12, 10, 12, 10)
        layout.setSpacing(4)

        # Title
        title = QLabel("ⓘ Shortcuts")
        title.setStyleSheet("font-size: 14px; font-weight: bold; color: #88ccff;")
        layout.addWidget(title)

        # Shortcuts
        shortcuts = [
            ("D", "Dual view"),
            ("1", "Single left"),
            ("2", "Single right"),
            ("←/→", "Switch camera"),
            ("⇧←/→", "Left cam (dual)"),
            ("⌃←/→", "Right cam (dual)"),
            ("F11", "Fullscreen"),
            ("Q", "Quit"),
        ]

        for key, desc in shortcuts:
            line = QLabel(f"<b>{key}</b>  {desc}")
            line.setStyleSheet("font-size: 12px;")
            layout.addWidget(line)

        self.info_panel.adjustSize()
        self.info_panel.move(5, 200 - self.info_panel.height() - 5)
        self.info_panel.hide()  # Hidden by default

        # Store original enter/leave events and override them
        container.enterEvent = self._on_info_hover_enter
        container.leaveEvent = self._on_info_hover_leave

        return container

    def _on_info_hover_enter(self, event: QEnterEvent):
        """Show the info panel when mouse enters the hover zone."""
        self.info_icon.setStyleSheet("""
            QLabel {
                color: rgba(255, 255, 255, 255);
                font-size: 24px;
                background: transparent;
            }
        """)
        self.info_panel.show()

    def _on_info_hover_leave(self, event):
        """Hide the info panel when mouse leaves the hover zone."""
        self.info_icon.setStyleSheet("""
            QLabel {
                color: rgba(255, 255, 255, 100);
                font-size: 24px;
                background: transparent;
            }
        """)
        self.info_panel.hide()

    def resizeEvent(self, event):
        """Reposition info overlay when window is resized."""
        super().resizeEvent(event)
        # Position in bottom-left corner with some padding
        if hasattr(self, "info_overlay"):
            margin = 20
            self.info_overlay.move(
                margin, self.height() - self.info_overlay.height() - margin
            )

    def _create_logo_label(self, width: int) -> QLabel:
        label = QLabel()
        label.setFixedWidth(width)
        label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        label.setStyleSheet("background-color: black;")

        # Load logo from same directory as script
        script_dir = os.path.dirname(os.path.abspath(__file__))
        logo_path = os.path.join(script_dir, LOGO_FILENAME)

        if os.path.exists(logo_path):
            pixmap = QPixmap(logo_path)
            # Scale logo to fit width while keeping aspect ratio
            scaled = pixmap.scaledToWidth(
                width - 20,  # Small padding
                Qt.TransformationMode.SmoothTransformation,
            )
            label.setPixmap(scaled)
            Log.debug(f"Logo loaded: {logo_path}")
        else:
            Log.warning(f"Logo not found: {logo_path}")

        return label

    def update_frames(self):
        """Update video feeds based on current layout mode."""
        # Left feed (shown in DUAL and SINGLE_LEFT modes)
        if self.layout_mode in (LayoutMode.DUAL, LayoutMode.SINGLE_LEFT):
            left_pixmap, left_has_signal = self.left_feed.read_frame()
            if left_has_signal and left_pixmap:
                scaled = left_pixmap.scaled(
                    self.left_label.size(),
                    Qt.AspectRatioMode.KeepAspectRatio,
                    Qt.TransformationMode.SmoothTransformation,
                )
                self.left_label.setPixmap(scaled)
            else:
                # Show animated no-signal frame
                self._show_no_signal(self.left_label)

        # Right feed (shown in DUAL and SINGLE_RIGHT modes)
        if self.layout_mode in (LayoutMode.DUAL, LayoutMode.SINGLE_RIGHT):
            right_pixmap, right_has_signal = self.right_feed.read_frame()
            if right_has_signal and right_pixmap:
                scaled = right_pixmap.scaled(
                    self.right_label.size(),
                    Qt.AspectRatioMode.KeepAspectRatio,
                    Qt.TransformationMode.SmoothTransformation,
                )
                self.right_label.setPixmap(scaled)
            else:
                # Show animated no-signal frame
                self._show_no_signal(self.right_label)

    def _show_no_signal(self, label: QLabel):
        """Display animated no-signal frame on a label."""
        size = label.size()
        if size.width() > 0 and size.height() > 0:
            frame = self._create_no_signal_frame(size.width(), size.height())
            frame_rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            h, w, ch = frame_rgb.shape
            qimg = QImage(frame_rgb.data, w, h, ch * w, QImage.Format.Format_RGB888)
            label.setPixmap(QPixmap.fromImage(qimg))

    def set_layout_mode(self, mode: LayoutMode):
        """Switch to a different layout mode."""
        self.layout_mode = mode

        if mode == LayoutMode.DUAL:
            # Show both feeds
            self.left_label.show()
            self.right_label.show()
            self.center_logo.show()
            self.left_spacer.setFixedWidth(SIDE_MARGIN)
            self.right_spacer.setFixedWidth(SIDE_MARGIN)
            Log.info(
                f"Layout: DUAL (Left: cam {self.left_camera_index}, Right: cam {self.right_camera_index})"
            )

        elif mode == LayoutMode.SINGLE_LEFT:
            # Show only left feed, centered
            self.left_label.show()
            self.right_label.hide()
            self.center_logo.hide()
            # Expand margins to center the single feed
            total_margin = SIDE_MARGIN * 2 + CENTER_GAP
            self.left_spacer.setFixedWidth(total_margin)
            self.right_spacer.setFixedWidth(total_margin)
            Log.info(f"Layout: SINGLE LEFT (cam {self.left_camera_index})")

        elif mode == LayoutMode.SINGLE_RIGHT:
            # Show only right feed, centered
            self.left_label.hide()
            self.right_label.show()
            self.center_logo.hide()
            # Expand margins to center the single feed
            total_margin = SIDE_MARGIN * 2 + CENTER_GAP
            self.left_spacer.setFixedWidth(total_margin)
            self.right_spacer.setFixedWidth(total_margin)
            Log.info(f"Layout: SINGLE RIGHT (cam {self.right_camera_index})")

    def switch_camera(self, feed: str, direction: int):
        """Switch camera index for a feed. direction: 1=next, -1=previous"""
        if feed == "left":
            current_idx = AVAILABLE_CAMERA_INDICES.index(self.left_camera_index)
            new_idx = (current_idx + direction) % len(AVAILABLE_CAMERA_INDICES)
            self.left_camera_index = AVAILABLE_CAMERA_INDICES[new_idx]
            self.left_feed.release()
            self.left_feed = CameraFeed(
                self.left_camera_index,
                self.test_mode,
                "LEFT",
                self.switch_signals,
                self.always_no_signal,
            )
            Log.info(f"Left camera switched to index {self.left_camera_index}")
        else:
            current_idx = AVAILABLE_CAMERA_INDICES.index(self.right_camera_index)
            new_idx = (current_idx + direction) % len(AVAILABLE_CAMERA_INDICES)
            self.right_camera_index = AVAILABLE_CAMERA_INDICES[new_idx]
            self.right_feed.release()
            self.right_feed = CameraFeed(
                self.right_camera_index,
                self.test_mode,
                "RIGHT",
                self.switch_signals,
                self.always_no_signal,
            )
            Log.info(f"Right camera switched to index {self.right_camera_index}")

    def keyPressEvent(self, event):
        key = event.key()
        modifiers = event.modifiers()

        # Fullscreen toggle
        if key in (Qt.Key.Key_F11, Qt.Key.Key_F):
            if self.isFullScreen():
                self.showNormal()
            else:
                self.showFullScreen()

        # Exit
        elif key == Qt.Key.Key_Escape:
            if self.isFullScreen():
                self.showNormal()
            else:
                self.close()

        elif key == Qt.Key.Key_Q:
            self.close()

        # Layout switching
        elif key == Qt.Key.Key_D:
            self.set_layout_mode(LayoutMode.DUAL)

        elif key == Qt.Key.Key_1:
            self.set_layout_mode(LayoutMode.SINGLE_LEFT)

        elif key == Qt.Key.Key_2:
            self.set_layout_mode(LayoutMode.SINGLE_RIGHT)

        # Camera switching with arrow keys
        elif key == Qt.Key.Key_Right:
            if modifiers & Qt.KeyboardModifier.ShiftModifier:
                self.switch_camera("left", 1)
            elif modifiers & Qt.KeyboardModifier.ControlModifier:
                self.switch_camera("right", 1)
            elif self.layout_mode == LayoutMode.SINGLE_LEFT:
                self.switch_camera("left", 1)
            elif self.layout_mode == LayoutMode.SINGLE_RIGHT:
                self.switch_camera("right", 1)

        elif key == Qt.Key.Key_Left:
            if modifiers & Qt.KeyboardModifier.ShiftModifier:
                self.switch_camera("left", -1)
            elif modifiers & Qt.KeyboardModifier.ControlModifier:
                self.switch_camera("right", -1)
            elif self.layout_mode == LayoutMode.SINGLE_LEFT:
                self.switch_camera("left", -1)
            elif self.layout_mode == LayoutMode.SINGLE_RIGHT:
                self.switch_camera("right", -1)

        # Test mode: toggle no-signal simulation
        elif key == Qt.Key.Key_N and self.test_mode:
            # Toggle signal for the active feed(s)
            if self.layout_mode == LayoutMode.SINGLE_LEFT:
                has_signal = self.left_feed.toggle_signal()
                Log.info(f"Left feed signal: {'ON' if has_signal else 'OFF'}")
            elif self.layout_mode == LayoutMode.SINGLE_RIGHT:
                has_signal = self.right_feed.toggle_signal()
                Log.info(f"Right feed signal: {'ON' if has_signal else 'OFF'}")
            else:  # DUAL mode - toggle both
                self.left_feed.toggle_signal()
                has_signal = self.right_feed.toggle_signal()
                Log.info(f"Both feeds signal: {'ON' if has_signal else 'OFF'}")

        else:
            super().keyPressEvent(event)

    def closeEvent(self, event):
        if self.timer.isActive():
            self.timer.stop()

        self.left_feed.release()
        self.right_feed.release()
        cv2.destroyAllWindows()
        event.accept()


def parse_args():
    parser = argparse.ArgumentParser(
        description="Dual Elgato Capture Card Viewer for Ultrawide Displays"
    )

    # Test mode group
    test_group = parser.add_mutually_exclusive_group()
    test_group.add_argument(
        "--mock",
        "-m",
        action="store_true",
        help="Run in test mode with mock video sources (animated pattern)",
    )
    test_group.add_argument(
        "--switch-signals",
        "-s",
        action="store_true",
        help="Test mode with signal cycling (10s signal, 10s no-signal)",
    )
    test_group.add_argument(
        "--no-signal",
        "-n",
        action="store_true",
        help="Test mode with always no-signal state",
    )

    # Verbose mode
    parser.add_argument(
        "--verbose",
        "-v",
        action="store_true",
        help="Enable verbose output with colored logging",
    )

    return parser.parse_args()


def main():
    args = parse_args()

    # Set verbose mode
    Log.set_verbose(args.verbose)

    # Determine test mode
    test_mode = args.mock or args.switch_signals or args.no_signal
    switch_signals = args.switch_signals
    always_no_signal = args.no_signal

    if test_mode:
        if always_no_signal:
            Log.header("NO-SIGNAL MODE - Always showing no-signal overlay", force=True)
        elif switch_signals:
            Log.header(
                "SWITCH-SIGNALS MODE - Cycling signal/no-signal every 10s", force=True
            )
        else:
            Log.header("MOCK MODE - Using mock video sources", force=True)
        Log.info("Layout: D=dual, 1=single left, 2=single right", force=True)
        Log.info("Camera: Arrow keys to switch camera index", force=True)

    app = QApplication(sys.argv)
    app.setStyle("Fusion")

    viewer = DualVideoViewer(
        test_mode=test_mode,
        switch_signals=switch_signals,
        always_no_signal=always_no_signal,
    )
    viewer.show()

    sys.exit(app.exec())


if __name__ == "__main__":
    main()
