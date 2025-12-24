"""
Main dual video viewer widget.

This is the core viewer component that displays two camera feeds side by side
with configurable spacing and various overlays.
"""

import os
import time
from collections import deque

import cv2
import numpy as np
from PyQt6.QtCore import Qt, QTimer
from PyQt6.QtGui import QImage, QPixmap
from PyQt6.QtWidgets import (
    QHBoxLayout,
    QLabel,
    QSizePolicy,
    QWidget,
)

from hdmi_viewer.config import (
    CENTER_GAP,
    CURSOR_HIDE_DELAY_MS,
    FULLSCREEN,
    LOGO_FILENAME,
    SIDE_MARGIN,
    TIMER_INTERVAL_MS,
    LayoutMode,
    get_all_enabled_inputs,
    get_all_input_configs,
    get_available_input_indices,
    get_left_input_index,
    get_right_input_index,
    reload_config,
)
from hdmi_viewer.log import Log
from hdmi_viewer.utils import get_resource_path
from hdmi_viewer.widgets.audio_panel import AudioIcon, AudioPanel
from hdmi_viewer.widgets.base import HoverIcon
from hdmi_viewer.widgets.overlays import (
    InfoIcon,
    InfoPanel,
    InputNameOverlay,
    ScreenSaver,
)
from hdmi_viewer.widgets.settings_panel import SettingsPanel
from hdmi_viewer.widgets.thumbnails import ThumbnailsPanel
from hdmi_viewer.worker import CameraWorker


class SettingsIcon(HoverIcon):
    """Settings gear icon widget."""

    def __init__(self, parent=None):
        super().__init__("⚙", parent)


class DualVideoViewer(QWidget):
    """Main dual video viewer widget."""

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
        self.input_configs = get_all_input_configs()
        self.enabled_inputs = get_all_enabled_inputs()

        # Current input selection index (for dual mode cycling)
        self.dual_input_idx = 0

        # Input indices (can be changed at runtime)
        self.left_input_index = get_left_input_index()
        self.right_input_index = get_right_input_index()

        # Latest frame data from workers (updated via signals)
        self._left_pixmap: QPixmap | None = None
        self._left_has_signal: bool = False
        self._right_pixmap: QPixmap | None = None
        self._right_has_signal: bool = False

        # Create camera workers (threaded)
        self.left_worker = CameraWorker(
            self.left_input_index, test_mode, "LEFT", switch_signals, always_no_signal
        )
        self.right_worker = CameraWorker(
            self.right_input_index, test_mode, "RIGHT", switch_signals, always_no_signal
        )

        # Connect worker signals
        self.left_worker.frame_ready.connect(self._on_left_frame)
        self.right_worker.frame_ready.connect(self._on_right_frame)

        # Start workers
        self.left_worker.start()
        self.right_worker.start()

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
        layout.addWidget(self.left_label, 1)
        layout.addWidget(self.center_logo)
        layout.addWidget(self.right_label, 1)
        layout.addWidget(self.right_spacer)

        self.setLayout(layout)

        # Create overlay widgets
        self._setup_overlays()

        # Timers
        self._setup_timers()

        # State variables
        self._setup_state()

        # Enable mouse tracking on all child widgets for cursor auto-hide
        self._enable_mouse_tracking_recursive(self)

        # Start fullscreen if configured
        if FULLSCREEN:
            self.showFullScreen()
        else:
            self.resize(1920, 600)

        # Start frame update timer
        self.timer.start(TIMER_INTERVAL_MS)

    def _enable_mouse_tracking_recursive(self, widget):
        """Enable mouse tracking on widget and all children."""
        widget.setMouseTracking(True)
        for child in widget.findChildren(QWidget):
            child.setMouseTracking(True)

    def _setup_overlays(self):
        """Set up all overlay widgets."""
        # Input name overlay
        self.input_name_overlay = InputNameOverlay(self)

        # Info icon and panel
        self.info_icon = InfoIcon(self)
        self.info_panel = InfoPanel(self)
        self.info_icon.hover_changed.connect(self._on_info_hover_changed)

        # Settings icon and panel
        self.settings_icon = SettingsIcon(self)
        self.settings_icon.mousePressEvent = self._on_settings_click
        self.settings_panel = SettingsPanel(self)
        self.settings_panel.config_reloaded.connect(self._on_config_reloaded)

        # Audio icon and panel
        self.audio_icon = AudioIcon(self)
        self.audio_icon.clicked.connect(self._toggle_audio_panel)
        self.audio_panel = AudioPanel(self)

        # Thumbnails panel
        self.thumbnails_panel = ThumbnailsPanel(self)
        self.thumbnails_panel.input_selected.connect(self.select_input_by_index)

        # Screensaver
        self.screensaver = ScreenSaver(self)

    def _setup_timers(self):
        """Set up all timers."""
        # Frame update timer
        self.timer = QTimer(self)
        self.timer.timeout.connect(self.update_frames)

        # Cursor hide timer
        self.cursor_hide_timer = QTimer(self)
        self.cursor_hide_timer.timeout.connect(self._hide_cursor)
        self.cursor_hide_timer.setSingleShot(True)
        self.setMouseTracking(True)

    def _setup_state(self):
        """Initialize state variables."""
        # Cursor state
        self.cursor_hidden = False

        # Shake detection for cursor reveal
        self._mouse_history = deque(maxlen=50)  # (timestamp, x, y)
        self._shake_threshold = 2  # Number of direction reversals to detect shake
        self._shake_time_window = 0.5  # Seconds to look back for shake detection
        self._min_shake_distance = 20  # Minimum pixels for a movement to count

        # Screensaver mode
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
        self.last_signal_state = {}

        # No-signal video state
        self._no_signal_cache = None
        self._no_signal_frame_idx = 0
        self._no_signal_video = None  # cv2.VideoCapture for MP4

    # =========================================================================
    # FRAME SIGNAL HANDLERS (from worker threads)
    # =========================================================================

    def _on_left_frame(self, pixmap: QPixmap | None, has_signal: bool):
        """Handle new frame from left camera worker."""
        self._left_pixmap = pixmap
        self._left_has_signal = has_signal
        self._check_auto_switch(self.left_input_index, has_signal)

    def _on_right_frame(self, pixmap: QPixmap | None, has_signal: bool):
        """Handle new frame from right camera worker."""
        self._right_pixmap = pixmap
        self._right_has_signal = has_signal
        self._check_auto_switch(self.right_input_index, has_signal)

    # =========================================================================
    # UI CREATION
    # =========================================================================

    def _create_video_label(self) -> QLabel:
        """Create a video display label."""
        label = QLabel()
        label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        label.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        label.setStyleSheet("background-color: black;")
        label.setMouseTracking(True)  # Enable mouse tracking for cursor auto-hide
        return label

    def _create_spacer(self, width: int) -> QLabel:
        """Create a spacer label."""
        spacer = QLabel()
        spacer.setFixedWidth(width)
        spacer.setStyleSheet("background-color: black;")
        spacer.setMouseTracking(True)  # Enable mouse tracking for cursor auto-hide
        return spacer

    def _create_logo_label(self, width: int) -> QLabel:
        """Create the center logo label."""
        label = QLabel()
        label.setFixedWidth(width)
        label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        label.setStyleSheet("background-color: black;")
        label.setMouseTracking(True)  # Enable mouse tracking for cursor auto-hide

        logo_path = get_resource_path(LOGO_FILENAME)
        if os.path.exists(logo_path):
            pixmap = QPixmap(logo_path)
            scaled = pixmap.scaledToWidth(
                width - 20,
                Qt.TransformationMode.SmoothTransformation,
            )
            label.setPixmap(scaled)
            Log.debug(f"Logo loaded: {logo_path}")
        else:
            Log.warning(f"Logo not found: {logo_path}")

        return label

    # =========================================================================
    # EVENT HANDLERS
    # =========================================================================

    def _on_info_hover_changed(self, hovering: bool):
        """Handle info icon hover state change."""
        if hovering:
            self.info_panel.show()
        else:
            self.info_panel.hide()

    def _on_settings_click(self, event):
        """Handle click on settings gear icon."""
        if self.settings_panel.isVisible():
            self.settings_panel.hide()
        else:
            self.settings_panel.refresh()
            x = (self.width() - self.settings_panel.width()) // 2
            y = (self.height() - self.settings_panel.height()) // 2
            self.settings_panel.move(x, y)
            self.settings_panel.show()
            self.settings_panel.raise_()

    def _toggle_audio_panel(self):
        """Toggle audio panel visibility."""
        if self.audio_panel.isVisible():
            self.audio_panel.hide()
        else:
            x = self.audio_icon.x()
            y = self.audio_icon.y() - self.audio_panel.height() - 10
            self.audio_panel.move(x, y)
            self.audio_panel.show()
            self.audio_panel.raise_()

    def _on_config_reloaded(self):
        """Handle configuration reload from settings panel."""
        self._reload_input_configs()

    # =========================================================================
    # CURSOR AUTO-HIDE
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
        self.cursor_hide_timer.start(CURSOR_HIDE_DELAY_MS)

    def mouseMoveEvent(self, event):
        """Handle mouse movement - show cursor on shake or restart timer."""
        pos = event.pos()
        current_time = time.time()

        # Record mouse position
        self._mouse_history.append((current_time, pos.x(), pos.y()))

        if self.cursor_hidden:
            # Only show cursor if we detect a shake
            if self._detect_shake():
                self._show_cursor()
        else:
            # Cursor visible - restart hide timer on any movement
            self._show_cursor()

        super().mouseMoveEvent(event)

    def _detect_shake(self) -> bool:
        """Detect if user is shaking the mouse (rapid direction reversals)."""
        if len(self._mouse_history) < 3:
            return False

        current_time = time.time()

        # Filter to recent movements within time window
        recent = [(t, x, y) for t, x, y in self._mouse_history
                  if current_time - t < self._shake_time_window]

        if len(recent) < 3:
            return False

        # Count horizontal direction reversals
        # Track cumulative movement from a reference point, reset on reversal
        reversals = 0
        last_direction = None
        reference_x = recent[0][1]
        cumulative_distance = 0

        for _, x, _ in recent[1:]:
            dx = x - reference_x
            
            if abs(dx) >= self._min_shake_distance:
                direction = 1 if dx > 0 else -1
                if last_direction is None:
                    # First significant movement - establish direction
                    last_direction = direction
                    reference_x = x
                elif direction != last_direction:
                    # Direction changed - count reversal and reset reference
                    reversals += 1
                    last_direction = direction
                    reference_x = x

        return reversals >= self._shake_threshold

    # =========================================================================
    # FREEZE FRAME
    # =========================================================================

    def _toggle_freeze(self, feed: str = "both"):
        """Toggle freeze frame for specified feed(s)."""
        if feed in ("left", "both"):
            self.freeze_left = not self.freeze_left
            if self.freeze_left:
                # Use cached frame from worker
                if self._left_has_signal and self._left_pixmap:
                    self.frozen_left_pixmap = self._left_pixmap
                Log.info("Left feed FROZEN")
            else:
                self.frozen_left_pixmap = None
                Log.info("Left feed UNFROZEN")

        if feed in ("right", "both"):
            self.freeze_right = not self.freeze_right
            if self.freeze_right:
                # Use cached frame from worker
                if self._right_has_signal and self._right_pixmap:
                    self.frozen_right_pixmap = self._right_pixmap
                Log.info("Right feed FROZEN")
            else:
                self.frozen_right_pixmap = None
                Log.info("Right feed UNFROZEN")

        # Show freeze indicator
        if self.freeze_left or self.freeze_right:
            self.input_name_overlay.show_name("❙❙ FROZEN")
        else:
            self.input_name_overlay.show_name("▶ LIVE")

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
            current_left_signal = self.last_signal_state.get(self.left_input_index, True)
            current_right_signal = self.last_signal_state.get(self.right_input_index, True)

            if not current_left_signal or not current_right_signal:
                Log.info(f"Auto-switching to input {input_index} (signal detected)")
                self.select_input_by_index(input_index)

    # =========================================================================
    # FRAME UPDATES
    # =========================================================================

    def update_frames(self):
        """Update video display from cached frames (workers update in background)."""
        left_has_signal = self._left_has_signal
        right_has_signal = self._right_has_signal

        # Left feed
        if self.layout_mode in (LayoutMode.DUAL, LayoutMode.SINGLE_LEFT):
            if self.freeze_left and self.frozen_left_pixmap:
                self._display_frame(self.left_label, self.frozen_left_pixmap)
                left_has_signal = True
            elif self._left_has_signal and self._left_pixmap:
                self._display_frame(self.left_label, self._left_pixmap)
            else:
                self._show_no_signal(self.left_label)

        # Right feed
        if self.layout_mode in (LayoutMode.DUAL, LayoutMode.SINGLE_RIGHT):
            if self.freeze_right and self.frozen_right_pixmap:
                self._display_frame(self.right_label, self.frozen_right_pixmap)
                right_has_signal = True
            elif self._right_has_signal and self._right_pixmap:
                self._display_frame(self.right_label, self._right_pixmap)
            else:
                self._show_no_signal(self.right_label)

        # Check for screensaver mode
        self._update_screensaver_state(left_has_signal, right_has_signal)

    def _display_frame(self, label: QLabel, pixmap: QPixmap):
        """Scale and display a frame on a label."""
        scaled = pixmap.scaled(
            label.size(),
            Qt.AspectRatioMode.KeepAspectRatio,
            Qt.TransformationMode.SmoothTransformation,
        )
        label.setPixmap(scaled)

    def _update_screensaver_state(self, left_has_signal: bool, right_has_signal: bool):
        """Update screensaver state based on signal status."""
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
                self.screensaver.hide()
                Log.info("Screensaver mode deactivated")

        if self.screensaver_active:
            self.screensaver.show_frame(self.width(), self.height())

    def _show_no_signal(self, label: QLabel):
        """Display animated no-signal frame on a label."""
        size = label.size()
        if size.width() > 0 and size.height() > 0:
            frame = self._create_no_signal_frame(size.width(), size.height())
            frame_rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            h, w, ch = frame_rgb.shape
            qimg = QImage(frame_rgb.data, w, h, ch * w, QImage.Format.Format_RGB888)
            label.setPixmap(QPixmap.fromImage(qimg))

    def _create_no_signal_frame(self, width: int, height: int) -> np.ndarray:
        """Generate no-signal frame from MP4 video loop."""
        cache_key = (width, height)

        # Check if we need to reload (size changed or not cached)
        if (
            not self._no_signal_cache
            or self._no_signal_cache.get("size") != cache_key
        ):
            video_path = get_resource_path("no_signal.mp4")
            if os.path.exists(video_path):
                # Load all frames from the video
                cap = cv2.VideoCapture(video_path)
                frames = []

                # Text settings
                text = "Please connect a source"
                font = cv2.FONT_HERSHEY_SIMPLEX
                font_scale = max(0.6, width / 1920)  # Scale font with resolution
                font_thickness = max(1, int(width / 960))
                text_color = (128, 128, 128)  # Grey

                # Calculate text size for positioning
                (text_w, text_h), baseline = cv2.getTextSize(text, font, font_scale, font_thickness)

                while True:
                    ret, frame = cap.read()
                    if not ret:
                        break

                    # Resize frame to fit the target size while maintaining aspect ratio
                    frame_h, frame_w = frame.shape[:2]
                    scale = min(width / frame_w, height / frame_h)
                    new_w = int(frame_w * scale)
                    new_h = int(frame_h * scale)

                    # Resize and center on black background
                    resized = cv2.resize(frame, (new_w, new_h), interpolation=cv2.INTER_LINEAR)

                    # Create black background and center the video
                    output = np.zeros((height, width, 3), dtype=np.uint8)
                    x_offset = (width - new_w) // 2
                    y_offset = (height - new_h) // 2
                    output[y_offset:y_offset + new_h, x_offset:x_offset + new_w] = resized

                    # Add text below the animation
                    text_x = (width - text_w) // 2
                    text_y = y_offset + new_h + text_h + 30  # 30px below video
                    if text_y < height - 10:  # Ensure text fits
                        cv2.putText(output, text, (text_x, text_y), font, font_scale, text_color, font_thickness, cv2.LINE_AA)

                    frames.append(output)

                cap.release()

                if frames:
                    self._no_signal_cache = {
                        "size": cache_key,
                        "frames": frames,
                        "num_frames": len(frames),
                    }
                    self._no_signal_frame_idx = 0
                else:
                    self._no_signal_cache = {"size": cache_key, "frames": None}
            else:
                self._no_signal_cache = {"size": cache_key, "frames": None}

        cache = self._no_signal_cache
        if cache.get("frames") is None:
            frame = np.zeros((height, width, 3), dtype=np.uint8)
            cv2.putText(
                frame, "No Signal", (width // 2 - 100, height // 2),
                cv2.FONT_HERSHEY_SIMPLEX, 1.5, (128, 128, 128), 2
            )
            return frame

        frame = cache["frames"][self._no_signal_frame_idx]
        self._no_signal_frame_idx = (self._no_signal_frame_idx + 1) % cache["num_frames"]
        return frame

    # =========================================================================
    # LAYOUT SWITCHING
    # =========================================================================

    def set_layout_mode(self, mode: LayoutMode):
        """Switch to a different layout mode."""
        self.layout_mode = mode

        if mode == LayoutMode.DUAL:
            self.left_label.show()
            self.right_label.show()
            self.center_logo.show()
            self.left_spacer.setFixedWidth(SIDE_MARGIN)
            self.right_spacer.setFixedWidth(SIDE_MARGIN)
            Log.info(f"Layout: DUAL (Left: {self.left_input_index}, Right: {self.right_input_index})")

        elif mode == LayoutMode.SINGLE_LEFT:
            self.left_label.show()
            self.right_label.hide()
            self.center_logo.hide()
            total_margin = SIDE_MARGIN * 2 + CENTER_GAP
            self.left_spacer.setFixedWidth(total_margin)
            self.right_spacer.setFixedWidth(total_margin)
            Log.info(f"Layout: SINGLE LEFT (input {self.left_input_index})")

        elif mode == LayoutMode.SINGLE_RIGHT:
            self.left_label.hide()
            self.right_label.show()
            self.center_logo.hide()
            total_margin = SIDE_MARGIN * 2 + CENTER_GAP
            self.left_spacer.setFixedWidth(total_margin)
            self.right_spacer.setFixedWidth(total_margin)
            Log.info(f"Layout: SINGLE RIGHT (input {self.right_input_index})")

    # =========================================================================
    # INPUT SWITCHING
    # =========================================================================

    def _get_input_name(self, index: int) -> str:
        """Get friendly name for an input index."""
        for inp in self.input_configs:
            if inp.index == index:
                return inp.name
        return f"Input {index}"

    def _switch_feed(self, feed: str, new_index: int):
        """Switch a feed to a new input index (thread-safe)."""
        if feed in ("left", "both"):
            self.left_input_index = new_index
            self.left_worker.switch_camera(new_index)
        if feed in ("right", "both"):
            self.right_input_index = new_index
            self.right_worker.switch_camera(new_index)

    def switch_input(self, feed: str, direction: int):
        """Switch input index for a feed. direction: 1=next, -1=previous"""
        available = get_available_input_indices()
        if not available:
            return

        current_index = self.left_input_index if feed == "left" else self.right_input_index
        try:
            current_idx = available.index(current_index)
        except ValueError:
            current_idx = 0
        new_index = available[(current_idx + direction) % len(available)]

        self._switch_feed(feed, new_index)
        input_name = self._get_input_name(new_index)
        Log.info(f"{feed.title()} input: {input_name} (index {new_index})")
        self.input_name_overlay.show_name(input_name)

    def switch_dual_inputs(self, direction: int):
        """Switch both inputs together in dual mode."""
        if not self.enabled_inputs:
            return

        self.dual_input_idx = (self.dual_input_idx + direction) % len(self.enabled_inputs)
        new_input = self.enabled_inputs[self.dual_input_idx]

        self._switch_feed("both", new_input.index)
        Log.info(f"Dual inputs: {new_input.name} (index {new_input.index})")
        self.input_name_overlay.show_name(new_input.name)

    def select_input_by_index(self, index: int):
        """Select an input directly by its index (0-3)."""
        target_input = next((inp for inp in self.enabled_inputs if inp.index == index), None)

        if target_input is None:
            Log.warning(f"Input {index + 1} is not available")
            return

        if self.layout_mode == LayoutMode.DUAL:
            self.dual_input_idx = self.enabled_inputs.index(target_input)
            self._switch_feed("both", target_input.index)
            Log.info(f"Dual inputs: {target_input.name} (index {target_input.index})")

        elif self.layout_mode == LayoutMode.SINGLE_LEFT:
            self._switch_feed("left", target_input.index)
            Log.info(f"Left input: {target_input.name} (index {target_input.index})")

        elif self.layout_mode == LayoutMode.SINGLE_RIGHT:
            self._switch_feed("right", target_input.index)
            Log.info(f"Right input: {target_input.name} (index {target_input.index})")

        self.input_name_overlay.show_name(target_input.name)

    def _reload_input_configs(self):
        """Reload input configurations and apply changes live."""
        reload_config()
        self.input_configs = get_all_input_configs()
        self.enabled_inputs = get_all_enabled_inputs()
        available = get_available_input_indices()

        # Check if current inputs are still valid, switch to first available if not
        if self.left_input_index not in available and available:
            self._switch_feed("left", available[0])
            Log.info(f"Left input switched to {self._get_input_name(available[0])}")

        if self.right_input_index not in available and available:
            self._switch_feed("right", available[0])
            Log.info(f"Right input switched to {self._get_input_name(available[0])}")

        if self.dual_input_idx >= len(self.enabled_inputs):
            self.dual_input_idx = 0

        Log.debug(f"Reloaded {len(self.enabled_inputs)} enabled inputs")

    # =========================================================================
    # RESIZE AND POSITIONING
    # =========================================================================

    def resizeEvent(self, event):
        """Reposition overlays when window is resized."""
        super().resizeEvent(event)
        margin = 20
        bottom_y = self.height() - 40 - margin

        # Position info icon
        self.info_icon.move(margin, bottom_y)

        # Position settings icon next to info
        self.settings_icon.move(margin + 40, bottom_y + 5)

        # Position audio icon next to settings
        self.audio_icon.move(margin + 80, bottom_y + 5)

        # Position info panel above icons
        self.info_panel.move(margin, bottom_y - self.info_panel.height() - 5)

        # Position audio panel
        if self.audio_panel.isVisible():
            self.audio_panel.move(margin + 80, bottom_y - self.audio_panel.height() - 10)

        # Center settings panel
        if self.settings_panel.isVisible():
            x = (self.width() - self.settings_panel.width()) // 2
            y = (self.height() - self.settings_panel.height()) // 2
            self.settings_panel.move(x, y)

        # Center thumbnails panel
        if self.thumbnails_panel.isVisible():
            x = (self.width() - self.thumbnails_panel.width()) // 2
            y = (self.height() - self.thumbnails_panel.height()) // 2
            self.thumbnails_panel.move(x, y)

    # =========================================================================
    # KEYBOARD SHORTCUTS
    # =========================================================================

    def keyPressEvent(self, event):
        """Handle keyboard shortcuts."""
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
            input_index = key - Qt.Key.Key_1
            self.select_input_by_index(input_index)

        # Freeze frame
        elif key == Qt.Key.Key_Space:
            self._toggle_freeze("both")

        # Input thumbnails panel
        elif key == Qt.Key.Key_T:
            if self.thumbnails_panel.isVisible():
                self.thumbnails_panel.hide()
            else:
                self.thumbnails_panel.show_centered(self.width(), self.height())

        # Toggle auto-switch
        elif key == Qt.Key.Key_A:
            self.auto_switch_enabled = not self.auto_switch_enabled
            state = "ON" if self.auto_switch_enabled else "OFF"
            Log.info(f"Auto-switch: {state}")
            self.input_name_overlay.show_name(f"Auto-switch: {state}")

        # Test mode: toggle no-signal simulation
        elif key == Qt.Key.Key_N and self.test_mode:
            if self.layout_mode == LayoutMode.SINGLE_LEFT:
                has_signal = self.left_worker.toggle_signal()
                Log.info(f"Left feed signal: {'ON' if has_signal else 'OFF'}")
            elif self.layout_mode == LayoutMode.SINGLE_RIGHT:
                has_signal = self.right_worker.toggle_signal()
                Log.info(f"Right feed signal: {'ON' if has_signal else 'OFF'}")
            else:
                self.left_worker.toggle_signal()
                has_signal = self.right_worker.toggle_signal()
                Log.info(f"Both feeds signal: {'ON' if has_signal else 'OFF'}")

        else:
            super().keyPressEvent(event)

    # =========================================================================
    # CLEANUP
    # =========================================================================

    def closeEvent(self, event):
        """Clean up resources on close."""
        if self.timer.isActive():
            self.timer.stop()

        # Stop worker threads
        self.left_worker.stop()
        self.right_worker.stop()

        cv2.destroyAllWindows()
        event.accept()
