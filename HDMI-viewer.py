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
import json
import os
import shutil
import sys
import time
from dataclasses import dataclass
from enum import Enum, auto
from typing import Optional

import cv2
import numpy as np
from PIL import Image, ImageDraw, ImageFont
from PyQt6.QtCore import QTimer, Qt, QPropertyAnimation, QEasingCurve, pyqtProperty
from PyQt6.QtGui import QEnterEvent, QImage, QPixmap, QColor, QPainter, QBrush
from PyQt6.QtWidgets import (
    QApplication,
    QFrame,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QPushButton,
    QScrollArea,
    QSizePolicy,
    QVBoxLayout,
    QWidget,
    QSlider,
    QGridLayout,
)


def get_resource_path(relative_path: str) -> str:
    """Get absolute path to resource, works for dev and for PyInstaller bundle."""
    if hasattr(sys, "_MEIPASS"):
        # PyInstaller creates a temp folder and stores path in _MEIPASS
        base_path = sys._MEIPASS
    else:
        base_path = os.path.dirname(os.path.abspath(__file__))
    return os.path.join(base_path, relative_path)


def get_user_data_path(filename: str) -> str:
    """Get path for user data files (settings that need to persist)."""
    if hasattr(sys, "_MEIPASS"):
        # When bundled, use user's home directory for writable files
        if sys.platform == "darwin":
            data_dir = os.path.expanduser("~/Library/Application Support/Space Presenter")
        elif sys.platform == "win32":
            data_dir = os.path.join(os.environ.get("APPDATA", ""), "Space Presenter")
        else:
            data_dir = os.path.expanduser("~/.config/hdmi-viewer")
        os.makedirs(data_dir, exist_ok=True)
        return os.path.join(data_dir, filename)
    else:
        # Development mode - use script directory
        return os.path.join(os.path.dirname(os.path.abspath(__file__)), filename)


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


@dataclass
class InputConfig:
    """Configuration for a single input."""

    index: int
    name: str
    enabled: bool = True
    default: bool = False


def load_settings() -> dict:
    """Load settings from settings.json file."""
    settings_path = get_user_data_path("settings.json")

    # If no user settings exist, copy default from bundle
    if not os.path.exists(settings_path):
        default_settings_path = get_resource_path("settings.json")
        if os.path.exists(default_settings_path):
            try:
                shutil.copy(default_settings_path, settings_path)
                Log.debug(f"Copied default settings to {settings_path}")
            except IOError:
                pass

    if os.path.exists(settings_path):
        try:
            with open(settings_path, "r") as f:
                settings = json.load(f)
            Log.debug(f"Settings loaded from {settings_path}")
            return settings
        except (json.JSONDecodeError, IOError) as e:
            Log.warning(f"Failed to load settings.json: {e}")

    # Return default settings if file doesn't exist or fails to load
    return {
        "inputs": [
            {"index": 0, "name": "Input 1", "enabled": True, "default": True},
            {"index": 1, "name": "Input 2", "enabled": True, "default": False},
            {"index": 2, "name": "Input 3", "enabled": True, "default": False},
            {"index": 3, "name": "Input 4", "enabled": False, "default": False},
        ]
    }


def save_settings(settings: dict):
    """Save settings to settings.json file."""
    settings_path = get_user_data_path("settings.json")

    try:
        with open(settings_path, "w") as f:
            json.dump(settings, f, indent=4)
        Log.debug(f"Settings saved to {settings_path}")
    except IOError as e:
        Log.error(f"Failed to save settings.json: {e}")


def get_input_configs(settings: dict) -> list[InputConfig]:
    """Parse input configurations from settings."""
    inputs = []
    for input_data in settings.get("inputs", []):
        inputs.append(
            InputConfig(
                index=input_data.get("index", 0),
                name=input_data.get("name", f"Input {input_data.get('index', 0)}"),
                enabled=input_data.get("enabled", True),
                default=input_data.get("default", False),
            )
        )
    return inputs


def get_enabled_inputs(inputs: list[InputConfig]) -> list[InputConfig]:
    """Get list of enabled inputs."""
    return [inp for inp in inputs if inp.enabled]


def get_default_input(inputs: list[InputConfig]) -> Optional[InputConfig]:
    """Get the default input, or first enabled input if no default."""
    for inp in inputs:
        if inp.default and inp.enabled:
            return inp
    # Fallback to first enabled input
    enabled = get_enabled_inputs(inputs)
    return enabled[0] if enabled else None


# =============================================================================
# CONFIGURATION - Loaded from settings.json
# =============================================================================

# Load settings
_settings = load_settings()
_input_configs = get_input_configs(_settings)
_enabled_inputs = get_enabled_inputs(_input_configs)
_default_input = get_default_input(_input_configs)

# Input indices (from your Cam Link Pro)
LEFT_INPUT_INDEX = _default_input.index if _default_input else 0
RIGHT_INPUT_INDEX = _default_input.index if _default_input else 0
AVAILABLE_INPUT_INDICES = [inp.index for inp in _enabled_inputs]

# Capture settings
TARGET_WIDTH = 3840
TARGET_HEIGHT = 2160
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


class ToggleSwitch(QWidget):
    """A custom iOS-style toggle switch widget."""

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
        """Set callback for when toggle state changes."""
        self._on_change_callback = callback

    def mousePressEvent(self, event):
        self._checked = not self._checked
        self._animation.setStartValue(self._circle_position)
        self._animation.setEndValue(22 if self._checked else 2)
        self._animation.start()
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


class NoSignalDetector:
    """
    Simple vision model to detect Elgato 'No Signal' screen.
    Uses multi-vector feature extraction and comparison against reference image.
    """
    
    # Feature vector size for comparison
    FEATURE_SIZE = 64
    # Similarity threshold (0-1, higher = more similar)
    SIMILARITY_THRESHOLD = 0.92
    
    def __init__(self):
        self.reference_features = None
        self._load_reference_image()
    
    def _load_reference_image(self):
        """Load and extract features from the reference no-signal image."""
        script_dir = os.path.dirname(os.path.abspath(__file__))
        ref_path = os.path.join(script_dir, "elgato_no_source.png")
        
        if os.path.exists(ref_path):
            ref_img = cv2.imread(ref_path)
            if ref_img is not None:
                self.reference_features = self._extract_features(ref_img)
                Log.success(f"NoSignalDetector: Loaded reference image, feature dim={len(self.reference_features)}")
            else:
                Log.warning("NoSignalDetector: Failed to read reference image")
        else:
            Log.warning(f"NoSignalDetector: Reference image not found at {ref_path}")
    
    def _extract_features(self, frame) -> np.ndarray:
        """
        Extract multi-vector features from a frame.
        Combines multiple feature types for robust matching.
        """
        # Resize to small fixed size for consistent comparison
        small = cv2.resize(frame, (64, 64))
        gray = cv2.cvtColor(small, cv2.COLOR_BGR2GRAY)
        
        features = []
        
        # 1. Color histogram features (HSV space is more robust)
        hsv = cv2.cvtColor(small, cv2.COLOR_BGR2HSV)
        for i, channel in enumerate(cv2.split(hsv)):
            hist = cv2.calcHist([channel], [0], None, [16], [0, 256])
            hist = cv2.normalize(hist, hist).flatten()
            features.extend(hist)
        
        # 2. Spatial intensity features (downsampled grid)
        grid = cv2.resize(gray, (8, 8)).flatten() / 255.0
        features.extend(grid)
        
        # 3. Edge density features (gradient magnitude)
        sobelx = cv2.Sobel(gray, cv2.CV_64F, 1, 0, ksize=3)
        sobely = cv2.Sobel(gray, cv2.CV_64F, 0, 1, ksize=3)
        magnitude = np.sqrt(sobelx**2 + sobely**2)
        edge_grid = cv2.resize(magnitude, (4, 4)).flatten()
        edge_grid = edge_grid / (edge_grid.max() + 1e-6)  # Normalize
        features.extend(edge_grid)
        
        # 4. Statistical moments
        features.append(np.mean(gray) / 255.0)
        features.append(np.std(gray) / 128.0)
        
        return np.array(features, dtype=np.float32)
    
    def _cosine_similarity(self, a: np.ndarray, b: np.ndarray) -> float:
        """Compute cosine similarity between two feature vectors."""
        dot = np.dot(a, b)
        norm_a = np.linalg.norm(a)
        norm_b = np.linalg.norm(b)
        if norm_a == 0 or norm_b == 0:
            return 0.0
        return dot / (norm_a * norm_b)
    
    def is_no_signal(self, frame) -> bool:
        """
        Detect if frame matches the no-signal reference.
        Returns True if the frame is likely a no-signal screen.
        """
        if self.reference_features is None:
            return False
        
        frame_features = self._extract_features(frame)
        similarity = self._cosine_similarity(frame_features, self.reference_features)
        
        return similarity >= self.SIMILARITY_THRESHOLD


# Shared detector instance (loaded once, used by all feeds)
_no_signal_detector = None

def get_no_signal_detector() -> NoSignalDetector:
    """Get or create the shared NoSignalDetector instance."""
    global _no_signal_detector
    if _no_signal_detector is None:
        _no_signal_detector = NoSignalDetector()
    return _no_signal_detector


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
        self.frame_counter = 0
        self.check_interval = 100  # Check every 100 frames (~3.3s at 30Hz)
        self._last_no_signal_result = False
        self.detector = get_no_signal_detector()

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
        Uses a vision model with multi-vector feature comparison against
        a reference image. Only checks every 100 frames (~3.3s at 30Hz).
        """
        self.frame_counter += 1
        
        # Only check every N frames for efficiency (4K 30Hz = check every ~3.3s)
        if self.frame_counter % self.check_interval != 0:
            return self._last_no_signal_result
        
        # Use the vision model detector
        self._last_no_signal_result = self.detector.is_no_signal(frame)
        
        return self._last_no_signal_result

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

        # Input configurations
        self.input_configs = _input_configs
        self.enabled_inputs = _enabled_inputs

        # Current input selection index (for dual mode cycling)
        self.dual_input_idx = 0  # Index into enabled_inputs list

        # Input indices (can be changed at runtime)
        self.left_input_index = LEFT_INPUT_INDEX
        self.right_input_index = RIGHT_INPUT_INDEX

        # Open both inputs
        self.left_feed = CameraFeed(
            self.left_input_index, test_mode, "LEFT", switch_signals, always_no_signal
        )
        self.right_feed = CameraFeed(
            self.right_input_index,
            test_mode,
            "RIGHT",
            switch_signals,
            always_no_signal,
        )

        # Window setup
        self.setWindowTitle("Space Presenter")
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

        # Settings gear icon (separate from info overlay for independent clicking)
        self.settings_icon = self._create_settings_icon()

        # Settings panel (hidden by default)
        self.settings_panel = self._create_settings_panel()

        # Input name overlay (shown briefly when switching inputs)
        self.input_name_label = self._create_input_name_overlay()
        self.input_name_timer = QTimer(self)
        self.input_name_timer.timeout.connect(self._hide_input_name)
        self.input_name_timer.setSingleShot(True)

        # Auto-hide cursor after inactivity
        self.cursor_hide_timer = QTimer(self)
        self.cursor_hide_timer.timeout.connect(self._hide_cursor)
        self.cursor_hide_timer.setSingleShot(True)
        self.cursor_hidden = False
        self.setMouseTracking(True)
        
        # Screensaver mode (when all inputs have no signal)
        self.screensaver_active = False
        self.no_signal_start_time = None
        self.screensaver_delay = 60  # Seconds before screensaver activates
        
        # Freeze frame feature
        self.freeze_left = False
        self.freeze_right = False
        self.frozen_left_pixmap = None
        self.frozen_right_pixmap = None
        
        # Auto-switch on signal
        self.auto_switch_enabled = True
        self.last_signal_state = {}  # Track signal state per input
        
        # Input thumbnails panel
        self.thumbnails_panel = self._create_thumbnails_panel()
        self.thumbnail_feeds = {}  # Will hold CameraFeed objects for thumbnails
        
        # Audio control panel
        self.audio_panel = self._create_audio_panel()
        self.audio_icon = self._create_audio_icon()

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

    def _create_input_name_overlay(self) -> QLabel:
        """Create an overlay label to show the input name when switching."""
        label = QLabel(self)
        label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        label.setStyleSheet(
            """
            QLabel {
                color: white;
                font-size: 48px;
                font-weight: bold;
                font-family: 'TT Interphases Pro', 'Helvetica Neue', Arial, sans-serif;
                background-color: rgba(0, 0, 0, 180);
                border-radius: 16px;
                padding: 20px 40px;
            }
            """
        )
        label.hide()
        return label

    def _show_input_name(self, name: str):
        """Show the input name overlay briefly."""
        self.input_name_label.setText(name)
        self.input_name_label.adjustSize()
        # Center the label
        x = (self.width() - self.input_name_label.width()) // 2
        y = (self.height() - self.input_name_label.height()) // 2
        self.input_name_label.move(x, y)
        self.input_name_label.show()
        self.input_name_label.raise_()
        # Hide after 1.5 seconds
        self.input_name_timer.start(1500)

    def _hide_input_name(self):
        """Hide the input name overlay."""
        self.input_name_label.hide()

    # =========================================================================
    # AUTO-HIDE CURSOR
    # =========================================================================
    
    def _hide_cursor(self):
        """Hide the mouse cursor."""
        self.setCursor(Qt.CursorShape.BlankCursor)
        self.cursor_hidden = True
    
    def _show_cursor(self):
        """Show the mouse cursor and restart hide timer."""
        if self.cursor_hidden:
            self.setCursor(Qt.CursorShape.ArrowCursor)
            self.cursor_hidden = False
        self.cursor_hide_timer.start(3000)  # Hide after 3 seconds
    
    def mouseMoveEvent(self, event):
        """Handle mouse movement - show cursor and restart timer."""
        self._show_cursor()
        super().mouseMoveEvent(event)

    # =========================================================================
    # SCREENSAVER MODE
    # =========================================================================
    
    def _create_screensaver_frame(self, width: int, height: int) -> np.ndarray:
        """Generate a screensaver frame with animated logo."""
        if not hasattr(self, '_screensaver_cache') or self._screensaver_cache.get('size') != (width, height):
            # Load logo for screensaver
            logo_path = get_resource_path(LOGO_FILENAME)
            self._screensaver_logo = None
            if os.path.exists(logo_path):
                logo = Image.open(logo_path).convert("RGBA")
                # Scale logo to reasonable size
                scale = min(width / logo.width, height / logo.height) * 0.3
                new_w = int(logo.width * scale)
                new_h = int(logo.height * scale)
                self._screensaver_logo = logo.resize((new_w, new_h), Image.Resampling.LANCZOS)
            self._screensaver_cache = {'size': (width, height)}
            self._screensaver_phase = 0
        
        # Create black frame
        frame = np.zeros((height, width, 3), dtype=np.uint8)
        
        if self._screensaver_logo:
            # Animate logo position (bouncing)
            self._screensaver_phase += 0.02
            logo_w, logo_h = self._screensaver_logo.size
            
            # Calculate bouncing position
            t = self._screensaver_phase
            x = int((width - logo_w) * (0.5 + 0.4 * np.sin(t * 0.7)))
            y = int((height - logo_h) * (0.5 + 0.4 * np.sin(t * 0.5)))
            
            # Paste logo onto frame
            logo_rgb = self._screensaver_logo.convert("RGB")
            logo_arr = np.array(logo_rgb)
            
            # Handle alpha blending
            if self._screensaver_logo.mode == "RGBA":
                alpha = np.array(self._screensaver_logo.split()[3]) / 255.0
                for c in range(3):
                    frame[y:y+logo_h, x:x+logo_w, c] = (
                        alpha * logo_arr[:, :, c] + (1 - alpha) * frame[y:y+logo_h, x:x+logo_w, c]
                    ).astype(np.uint8)
            else:
                frame[y:y+logo_h, x:x+logo_w] = logo_arr
        
        return frame

    # =========================================================================
    # FREEZE FRAME
    # =========================================================================
    
    def _toggle_freeze(self, feed: str = "both"):
        """Toggle freeze frame for specified feed(s)."""
        if feed in ("left", "both"):
            self.freeze_left = not self.freeze_left
            if self.freeze_left:
                # Capture current frame
                pixmap, has_signal = self.left_feed.read_frame()
                if has_signal and pixmap:
                    self.frozen_left_pixmap = pixmap
                Log.info("Left feed FROZEN")
            else:
                self.frozen_left_pixmap = None
                Log.info("Left feed UNFROZEN")
        
        if feed in ("right", "both"):
            self.freeze_right = not self.freeze_right
            if self.freeze_right:
                pixmap, has_signal = self.right_feed.read_frame()
                if has_signal and pixmap:
                    self.frozen_right_pixmap = pixmap
                Log.info("Right feed FROZEN")
            else:
                self.frozen_right_pixmap = None
                Log.info("Right feed UNFROZEN")
        
        # Show freeze indicator
        if self.freeze_left or self.freeze_right:
            self._show_input_name("❙❙ FROZEN")
        else:
            self._show_input_name("▶ LIVE")

    # =========================================================================
    # INPUT THUMBNAILS PANEL
    # =========================================================================
    
    def _create_thumbnails_panel(self) -> QFrame:
        """Create a panel showing thumbnails of all inputs."""
        panel = QFrame(self)
        panel.setFixedSize(320, 200)
        panel.setStyleSheet("""
            QFrame {
                background-color: rgba(30, 30, 30, 230);
                border-radius: 12px;
                border: 1px solid rgba(255, 255, 255, 30);
            }
        """)
        
        layout = QVBoxLayout(panel)
        layout.setContentsMargins(10, 10, 10, 10)
        layout.setSpacing(8)
        
        # Title
        title = QLabel("⊞ Inputs")
        title.setStyleSheet("color: rgba(255, 255, 255, 200); font-size: 14px; font-weight: bold; background: transparent;")
        layout.addWidget(title)
        
        # Grid for thumbnails
        grid = QGridLayout()
        grid.setSpacing(6)
        
        self.thumbnail_labels = {}
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
            thumb_frame.mousePressEvent = lambda e, idx=i: self._on_thumbnail_click(idx)
            
            thumb_layout = QVBoxLayout(thumb_frame)
            thumb_layout.setContentsMargins(4, 4, 4, 4)
            
            # Thumbnail image
            thumb_label = QLabel()
            thumb_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
            thumb_label.setStyleSheet("background: transparent;")
            thumb_layout.addWidget(thumb_label)
            
            # Input name
            name_label = QLabel(f"Input {i + 1}")
            name_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
            name_label.setStyleSheet("color: white; font-size: 10px; background: transparent;")
            thumb_layout.addWidget(name_label)
            
            self.thumbnail_labels[i] = {'frame': thumb_frame, 'thumb': thumb_label, 'name': name_label}
            grid.addWidget(thumb_frame, i // 2, i % 2)
        
        layout.addLayout(grid)
        panel.hide()
        return panel
    
    def _on_thumbnail_click(self, index: int):
        """Handle click on a thumbnail to switch to that input."""
        self.select_input_by_index(index)
        self.thumbnails_panel.hide()
    
    def _toggle_thumbnails_panel(self):
        """Toggle visibility of thumbnails panel."""
        if self.thumbnails_panel.isVisible():
            self.thumbnails_panel.hide()
        else:
            # Position in center
            x = (self.width() - self.thumbnails_panel.width()) // 2
            y = (self.height() - self.thumbnails_panel.height()) // 2
            self.thumbnails_panel.move(x, y)
            self.thumbnails_panel.show()
            self.thumbnails_panel.raise_()
    
    def _update_thumbnails(self):
        """Update thumbnail previews (called periodically)."""
        if not self.thumbnails_panel.isVisible():
            return
        
        for i, config in enumerate(self.input_configs):
            if i in self.thumbnail_labels:
                # Update name
                self.thumbnail_labels[i]['name'].setText(config.name)
                
                # For now, show a placeholder or indicator
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

    # =========================================================================
    # AUDIO CONTROL PANEL
    # =========================================================================
    
    def _create_audio_icon(self) -> QLabel:
        """Create the audio icon button."""
        icon = QLabel("♪", self)
        icon.setStyleSheet("""
            QLabel {
                color: rgba(255, 255, 255, 100);
                font-size: 24px;
                background: transparent;
            }
        """)
        icon.adjustSize()
        icon.setCursor(Qt.CursorShape.PointingHandCursor)
        icon.mousePressEvent = self._on_audio_click
        icon.enterEvent = self._on_audio_hover_enter
        icon.leaveEvent = self._on_audio_hover_leave
        return icon
    
    def _on_audio_hover_enter(self, event):
        """Highlight audio icon on hover."""
        self.audio_icon.setStyleSheet("""
            QLabel {
                color: rgba(255, 255, 255, 255);
                font-size: 24px;
                background: transparent;
            }
        """)
    
    def _on_audio_hover_leave(self, event):
        """Dim audio icon when not hovering."""
        self.audio_icon.setStyleSheet("""
            QLabel {
                color: rgba(255, 255, 255, 100);
                font-size: 24px;
                background: transparent;
            }
        """)
    
    def _on_audio_click(self, event):
        """Toggle audio panel visibility."""
        if self.audio_panel.isVisible():
            self.audio_panel.hide()
        else:
            # Position near the audio icon
            x = self.audio_icon.x()
            y = self.audio_icon.y() - self.audio_panel.height() - 10
            self.audio_panel.move(x, y)
            self.audio_panel.show()
            self.audio_panel.raise_()
    
    def _create_audio_panel(self) -> QFrame:
        """Create the audio control panel with sliders."""
        panel = QFrame(self)
        panel.setFixedSize(250, 180)
        panel.setStyleSheet("""
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
        panel.setObjectName("audioPanel")
        
        layout = QVBoxLayout(panel)
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
        panel.hide()
        return panel
    
    def _on_input_volume_changed(self, value: int):
        """Handle input volume slider change."""
        self.input_volume_value.setText(f"{value}%")
        # Note: Actual audio control would require platform-specific implementation
        Log.debug(f"Input volume: {value}%")
    
    def _on_system_volume_changed(self, value: int):
        """Handle system volume slider change."""
        self.system_volume_value.setText(f"{value}%")
        # Note: Actual system volume control would require platform-specific implementation
        Log.debug(f"System volume: {value}%")
    
    def _on_input_mute_toggle(self):
        """Toggle input audio mute."""
        muted = self.input_mute_btn.isChecked()
        self.input_mute_btn.setText("♪ Unmute Input" if muted else "⊘ Mute Input")
        Log.info(f"Input audio: {'MUTED' if muted else 'UNMUTED'}")
    
    def _on_system_mute_toggle(self):
        """Toggle system audio mute."""
        muted = self.system_mute_btn.isChecked()
        self.system_mute_btn.setText("♪ Unmute System" if muted else "⊘ Mute System")
        Log.info(f"System audio: {'MUTED' if muted else 'UNMUTED'}")

    # =========================================================================
    # AUTO-SWITCH ON SIGNAL
    # =========================================================================
    
    def _check_auto_switch(self, input_index: int, has_signal: bool):
        """Check if we should auto-switch to an input that just got signal."""
        if not self.auto_switch_enabled:
            return
        
        prev_state = self.last_signal_state.get(input_index, False)
        self.last_signal_state[input_index] = has_signal
        
        # If signal just appeared (was False, now True)
        if has_signal and not prev_state:
            # Check if current inputs have no signal
            current_left_signal = self.last_signal_state.get(self.left_input_index, True)
            current_right_signal = self.last_signal_state.get(self.right_input_index, True)
            
            # Only auto-switch if current input has no signal
            if not current_left_signal or not current_right_signal:
                Log.info(f"Auto-switching to input {input_index} (signal detected)")
                self.select_input_by_index(input_index)

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
            icon_path = get_resource_path("no_signal_icon.png")
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
        # Container that handles hover events for info only
        container = QFrame(self)
        container.setFixedSize(35, 40)  # Small hover area just for info icon
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
        self.info_icon.move(5, 5)

        # The actual info panel (hidden by default) - attached to main widget, not container
        self.info_panel = QFrame(self)
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

        self.info_panel.adjustSize()
        self.info_panel.hide()  # Hidden by default

        # Store original enter/leave events and override them
        container.enterEvent = self._on_info_hover_enter
        container.leaveEvent = self._on_info_hover_leave

        return container

    def _create_settings_icon(self) -> QLabel:
        """Create the settings gear icon."""
        icon = QLabel("⚙", self)
        icon.setStyleSheet("""
            QLabel {
                color: rgba(255, 255, 255, 100);
                font-size: 24px;
                background: transparent;
            }
        """)
        icon.adjustSize()
        icon.setCursor(Qt.CursorShape.PointingHandCursor)
        icon.mousePressEvent = self._on_settings_click
        icon.enterEvent = self._on_settings_hover_enter
        icon.leaveEvent = self._on_settings_hover_leave
        return icon

    def _on_settings_hover_enter(self, event):
        """Highlight gear icon on hover."""
        self.settings_icon.setStyleSheet("""
            QLabel {
                color: rgba(255, 255, 255, 255);
                font-size: 24px;
                background: transparent;
            }
        """)

    def _on_settings_hover_leave(self, event):
        """Dim gear icon when not hovering."""
        self.settings_icon.setStyleSheet("""
            QLabel {
                color: rgba(255, 255, 255, 100);
                font-size: 24px;
                background: transparent;
            }
        """)

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
        """Reposition overlays when window is resized."""
        super().resizeEvent(event)
        margin = 20
        bottom_y = self.height() - 40 - margin

        # Position info overlay in bottom-left corner
        if hasattr(self, "info_overlay"):
            self.info_overlay.move(margin, bottom_y)

        # Position settings icon next to info overlay
        if hasattr(self, "settings_icon"):
            self.settings_icon.move(margin + 40, bottom_y + 5)

        # Position audio icon next to settings
        if hasattr(self, "audio_icon"):
            self.audio_icon.move(margin + 80, bottom_y + 5)

        # Position info panel above the icons
        if hasattr(self, "info_panel"):
            self.info_panel.move(margin, bottom_y - self.info_panel.height() - 5)
        
        # Position audio panel above audio icon
        if hasattr(self, "audio_panel") and self.audio_panel.isVisible():
            self.audio_panel.move(margin + 80, bottom_y - self.audio_panel.height() - 10)

        # Center settings panel
        if hasattr(self, "settings_panel") and self.settings_panel.isVisible():
            x = (self.width() - self.settings_panel.width()) // 2
            y = (self.height() - self.settings_panel.height()) // 2
            self.settings_panel.move(x, y)
        
        # Center thumbnails panel
        if hasattr(self, "thumbnails_panel") and self.thumbnails_panel.isVisible():
            x = (self.width() - self.thumbnails_panel.width()) // 2
            y = (self.height() - self.thumbnails_panel.height()) // 2
            self.thumbnails_panel.move(x, y)

    def _on_settings_click(self, event):
        """Handle click on settings gear icon."""
        if self.settings_panel.isVisible():
            self.settings_panel.hide()
        else:
            self._refresh_settings_panel()
            # Center the panel
            x = (self.width() - self.settings_panel.width()) // 2
            y = (self.height() - self.settings_panel.height()) // 2
            self.settings_panel.move(x, y)
            self.settings_panel.show()
            self.settings_panel.raise_()

    def _create_settings_panel(self) -> QFrame:
        """Create the settings panel with input configuration."""
        panel = QFrame(self)
        panel.setFixedSize(400, 450)
        panel.setStyleSheet("""
            QFrame#settingsPanel {
                background-color: rgba(30, 30, 30, 240);
                border-radius: 12px;
                border: 1px solid rgba(255, 255, 255, 30);
            }
            QLabel {
                color: white;
                font-family: 'TT Interphases Pro', Arial, sans-serif;
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
        panel.setObjectName("settingsPanel")

        layout = QVBoxLayout(panel)
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
        close_btn.clicked.connect(lambda: panel.hide())
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

        # Store references to input widgets
        self.input_widgets = []

        scroll.setWidget(scroll_content)
        layout.addWidget(scroll)

        panel.hide()
        return panel

    def _refresh_settings_panel(self):
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
                lambda text, idx=i: self._on_input_name_changed(idx, text)
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
            enabled_toggle.setOnChange(
                lambda checked, idx=i: self._on_input_enabled_changed(idx, checked)
            )

            # Default toggle
            default_label = QLabel("Default:")
            default_label.setStyleSheet("font-size: 14px;")
            default_toggle = ToggleSwitch(input_data.get("default", False))
            default_toggle.setOnChange(
                lambda checked, idx=i: self._on_input_default_changed(idx, checked)
            )

            toggle_row.addWidget(enabled_label)
            toggle_row.addWidget(enabled_toggle)
            toggle_row.addSpacing(20)
            toggle_row.addWidget(default_label)
            toggle_row.addWidget(default_toggle)
            toggle_row.addStretch()
            input_layout.addLayout(toggle_row)

            self.inputs_layout.addWidget(input_frame)
            self.input_widgets.append(
                {
                    "name_edit": name_edit,
                    "enabled_toggle": enabled_toggle,
                    "default_toggle": default_toggle,
                }
            )

        self.inputs_layout.addStretch()

    def _on_input_name_changed(self, index: int, name: str):
        """Handle input name change."""
        settings = load_settings()
        if index < len(settings.get("inputs", [])):
            settings["inputs"][index]["name"] = name
            save_settings(settings)
            # Update local config
            if index < len(self.input_configs):
                self.input_configs[index] = InputConfig(
                    index=self.input_configs[index].index,
                    name=name,
                    enabled=self.input_configs[index].enabled,
                    default=self.input_configs[index].default,
                )

    def _on_input_enabled_changed(self, index: int, enabled: bool):
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
            self._reload_input_configs()

    def _on_input_default_changed(self, index: int, is_default: bool):
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
        self._reload_input_configs()

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
                    if hasattr(self, "input_widgets") and i < len(self.input_widgets):
                        self.input_widgets[i]["default_toggle"].setChecked(True)
                    break

    def _reload_input_configs(self):
        """Reload input configurations from settings and apply changes live."""
        global _input_configs, _enabled_inputs, _default_input
        global AVAILABLE_INPUT_INDICES

        settings = load_settings()
        self.input_configs = get_input_configs(settings)
        self.enabled_inputs = get_enabled_inputs(self.input_configs)

        _input_configs = self.input_configs
        _enabled_inputs = self.enabled_inputs
        _default_input = get_default_input(self.input_configs)
        AVAILABLE_INPUT_INDICES = [inp.index for inp in _enabled_inputs]

        # Check if current inputs are still valid, switch if not
        left_valid = self.left_input_index in AVAILABLE_INPUT_INDICES
        right_valid = self.right_input_index in AVAILABLE_INPUT_INDICES

        if not left_valid and len(AVAILABLE_INPUT_INDICES) > 0:
            # Switch to first available input
            new_index = AVAILABLE_INPUT_INDICES[0]
            self.left_input_index = new_index
            self.left_feed.release()
            self.left_feed = CameraFeed(
                self.left_input_index,
                self.test_mode,
                "LEFT",
                self.switch_signals,
                self.always_no_signal,
            )
            Log.info(f"Left input switched to {self._get_input_name(new_index)}")

        if not right_valid and len(AVAILABLE_INPUT_INDICES) > 0:
            # Switch to first available input
            new_index = AVAILABLE_INPUT_INDICES[0]
            self.right_input_index = new_index
            self.right_feed.release()
            self.right_feed = CameraFeed(
                self.right_input_index,
                self.test_mode,
                "RIGHT",
                self.switch_signals,
                self.always_no_signal,
            )
            Log.info(f"Right input switched to {self._get_input_name(new_index)}")

        # Reset dual camera index if needed
        if self.dual_input_idx >= len(self.enabled_inputs):
            self.dual_input_idx = 0

        Log.debug(f"Reloaded {len(self.enabled_inputs)} enabled inputs")

    def _create_logo_label(self, width: int) -> QLabel:
        label = QLabel()
        label.setFixedWidth(width)
        label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        label.setStyleSheet("background-color: black;")

        # Load logo from resources
        logo_path = get_resource_path(LOGO_FILENAME)

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
        left_has_signal = False
        right_has_signal = False
        
        # Left feed (shown in DUAL and SINGLE_LEFT modes)
        if self.layout_mode in (LayoutMode.DUAL, LayoutMode.SINGLE_LEFT):
            if self.freeze_left and self.frozen_left_pixmap:
                # Show frozen frame
                scaled = self.frozen_left_pixmap.scaled(
                    self.left_label.size(),
                    Qt.AspectRatioMode.KeepAspectRatio,
                    Qt.TransformationMode.SmoothTransformation,
                )
                self.left_label.setPixmap(scaled)
                left_has_signal = True
            else:
                left_pixmap, left_has_signal = self.left_feed.read_frame()
                self._check_auto_switch(self.left_input_index, left_has_signal)
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
            if self.freeze_right and self.frozen_right_pixmap:
                # Show frozen frame
                scaled = self.frozen_right_pixmap.scaled(
                    self.right_label.size(),
                    Qt.AspectRatioMode.KeepAspectRatio,
                    Qt.TransformationMode.SmoothTransformation,
                )
                self.right_label.setPixmap(scaled)
                right_has_signal = True
            else:
                right_pixmap, right_has_signal = self.right_feed.read_frame()
                self._check_auto_switch(self.right_input_index, right_has_signal)
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
        
        # Check for screensaver mode (all inputs have no signal)
        if not left_has_signal and not right_has_signal:
            if self.no_signal_start_time is None:
                self.no_signal_start_time = time.time()
            elif time.time() - self.no_signal_start_time > self.screensaver_delay:
                if not self.screensaver_active:
                    self.screensaver_active = True
                    Log.info("Screensaver mode activated")
        else:
            self.no_signal_start_time = None
            if self.screensaver_active:
                self.screensaver_active = False
                Log.info("Screensaver mode deactivated")
        
        # Show screensaver if active
        if self.screensaver_active:
            self._show_screensaver()
        
        # Update thumbnails if visible
        self._update_thumbnails()

    def _show_screensaver(self):
        """Display screensaver animation on both feeds."""
        for label in [self.left_label, self.right_label]:
            if label.isVisible():
                size = label.size()
                if size.width() > 0 and size.height() > 0:
                    frame = self._create_screensaver_frame(size.width(), size.height())
                    frame_rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
                    h, w, ch = frame_rgb.shape
                    qimg = QImage(frame_rgb.data, w, h, ch * w, QImage.Format.Format_RGB888)
                    label.setPixmap(QPixmap.fromImage(qimg))

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
                f"Layout: DUAL (Left: {self.left_input_index}, Right: {self.right_input_index})"
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
            Log.info(f"Layout: SINGLE LEFT (input {self.left_input_index})")

        elif mode == LayoutMode.SINGLE_RIGHT:
            # Show only right feed, centered
            self.left_label.hide()
            self.right_label.show()
            self.center_logo.hide()
            # Expand margins to center the single feed
            total_margin = SIDE_MARGIN * 2 + CENTER_GAP
            self.left_spacer.setFixedWidth(total_margin)
            self.right_spacer.setFixedWidth(total_margin)
            Log.info(f"Layout: SINGLE RIGHT (input {self.right_input_index})")

    def _get_input_name(self, index: int) -> str:
        """Get friendly name for an input index."""
        for inp in self.input_configs:
            if inp.index == index:
                return inp.name
        return f"Input {index}"

    def switch_input(self, feed: str, direction: int):
        """Switch input index for a feed. direction: 1=next, -1=previous"""
        if len(AVAILABLE_INPUT_INDICES) == 0:
            return

        if feed == "left":
            try:
                current_idx = AVAILABLE_INPUT_INDICES.index(self.left_input_index)
            except ValueError:
                current_idx = 0
            new_idx = (current_idx + direction) % len(AVAILABLE_INPUT_INDICES)
            self.left_input_index = AVAILABLE_INPUT_INDICES[new_idx]
            self.left_feed.release()
            self.left_feed = CameraFeed(
                self.left_input_index,
                self.test_mode,
                "LEFT",
                self.switch_signals,
                self.always_no_signal,
            )
            input_name = self._get_input_name(self.left_input_index)
            Log.info(f"Left input: {input_name} (index {self.left_input_index})")
            self._show_input_name(input_name)
        else:
            try:
                current_idx = AVAILABLE_INPUT_INDICES.index(self.right_input_index)
            except ValueError:
                current_idx = 0
            new_idx = (current_idx + direction) % len(AVAILABLE_INPUT_INDICES)
            self.right_input_index = AVAILABLE_INPUT_INDICES[new_idx]
            self.right_feed.release()
            self.right_feed = CameraFeed(
                self.right_input_index,
                self.test_mode,
                "RIGHT",
                self.switch_signals,
                self.always_no_signal,
            )
            input_name = self._get_input_name(self.right_input_index)
            Log.info(f"Right input: {input_name} (index {self.right_input_index})")
            self._show_input_name(input_name)

    def switch_dual_inputs(self, direction: int):
        """Switch both inputs together in dual mode. direction: 1=next, -1=previous"""
        if len(self.enabled_inputs) == 0:
            return

        self.dual_input_idx = (self.dual_input_idx + direction) % len(
            self.enabled_inputs
        )
        new_input = self.enabled_inputs[self.dual_input_idx]

        # Update both feeds to the same input
        self.left_input_index = new_input.index
        self.right_input_index = new_input.index

        self.left_feed.release()
        self.right_feed.release()

        self.left_feed = CameraFeed(
            self.left_input_index,
            self.test_mode,
            "LEFT",
            self.switch_signals,
            self.always_no_signal,
        )
        self.right_feed = CameraFeed(
            self.right_input_index,
            self.test_mode,
            "RIGHT",
            self.switch_signals,
            self.always_no_signal,
        )

        Log.info(f"Dual inputs: {new_input.name} (index {new_input.index})")
        self._show_input_name(new_input.name)

    def select_input_by_index(self, index: int):
        """Select an input directly by its index (0-3)."""
        # Find the input config with this index
        target_input = None
        for inp in self.enabled_inputs:
            if inp.index == index:
                target_input = inp
                break

        if target_input is None:
            # Input not enabled or doesn't exist
            Log.warning(f"Input {index + 1} is not available")
            return

        if self.layout_mode == LayoutMode.DUAL:
            # In dual mode, switch both feeds to the selected input
            self.left_input_index = target_input.index
            self.right_input_index = target_input.index
            self.dual_input_idx = self.enabled_inputs.index(target_input)

            self.left_feed.release()
            self.right_feed.release()

            self.left_feed = CameraFeed(
                self.left_input_index,
                self.test_mode,
                "LEFT",
                self.switch_signals,
                self.always_no_signal,
            )
            self.right_feed = CameraFeed(
                self.right_input_index,
                self.test_mode,
                "RIGHT",
                self.switch_signals,
                self.always_no_signal,
            )

            Log.info(f"Dual inputs: {target_input.name} (index {target_input.index})")

        elif self.layout_mode == LayoutMode.SINGLE_LEFT:
            self.left_input_index = target_input.index
            self.left_input_idx = self.enabled_inputs.index(target_input)

            self.left_feed.release()
            self.left_feed = CameraFeed(
                self.left_input_index,
                self.test_mode,
                "LEFT",
                self.switch_signals,
                self.always_no_signal,
            )

            Log.info(f"Left input: {target_input.name} (index {target_input.index})")

        elif self.layout_mode == LayoutMode.SINGLE_RIGHT:
            self.right_input_index = target_input.index
            self.right_input_idx = self.enabled_inputs.index(target_input)

            self.right_feed.release()
            self.right_feed = CameraFeed(
                self.right_input_index,
                self.test_mode,
                "RIGHT",
                self.switch_signals,
                self.always_no_signal,
            )

            Log.info(f"Right input: {target_input.name} (index {target_input.index})")

        self._show_input_name(target_input.name)

    def keyPressEvent(self, event):
        key = event.key()

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

        elif key == Qt.Key.Key_L:
            self.set_layout_mode(LayoutMode.SINGLE_LEFT)

        elif key == Qt.Key.Key_R:
            self.set_layout_mode(LayoutMode.SINGLE_RIGHT)

        # Direct input selection with 1-4
        elif key in (Qt.Key.Key_1, Qt.Key.Key_2, Qt.Key.Key_3, Qt.Key.Key_4):
            input_index = key - Qt.Key.Key_1  # 0-3
            self.select_input_by_index(input_index)

        # Freeze frame toggle (Space key)
        elif key == Qt.Key.Key_Space:
            self._toggle_freeze("both")
        
        # Input thumbnails panel (T key)
        elif key == Qt.Key.Key_T:
            self._toggle_thumbnails_panel()
        
        # Toggle auto-switch (A key)
        elif key == Qt.Key.Key_A:
            self.auto_switch_enabled = not self.auto_switch_enabled
            state = "ON" if self.auto_switch_enabled else "OFF"
            Log.info(f"Auto-switch: {state}")
            self._show_input_name(f"Auto-switch: {state}")

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
