"""
Camera feed handling for video capture.

Supports both real camera capture via OpenCV and mock sources for testing.
"""

import sys

import cv2
from PyQt6.QtGui import QImage, QPixmap

from hdmi_viewer.config import TARGET_FPS, TARGET_HEIGHT, TARGET_WIDTH
from hdmi_viewer.detection import get_no_signal_detector
from hdmi_viewer.log import Log


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
        self.check_interval = 5  # Check every 5 frames for responsive detection
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
                f"Mock camera {camera_index} ({label}) [{mode_str}]: "
                f"{TARGET_WIDTH}x{TARGET_HEIGHT} @ {TARGET_FPS} FPS"
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
        """Open a camera with platform-specific backend."""
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
        
        Uses smart detection:
        - If currently no-signal: quick check if still dark grey
        - If currently has signal: quick check if became dark, then full check
        - Full template matching only when state might have changed
        - Periodic full check every 60 frames when in no-signal state
        """
        self.frame_counter += 1

        # Only check every N frames for efficiency
        if self.frame_counter % self.check_interval != 0:
            return self._last_no_signal_result

        if self._last_no_signal_result:
            # Currently in no-signal state - quick check if still dark grey
            still_no_signal = self.detector.quick_check_still_no_signal(frame)

            # Force full check every 60 frames (~2 seconds) even if quick check passes
            force_full_check = (self.frame_counter % 60 == 0)

            if still_no_signal and not force_full_check:
                # Still dark grey, no need for full check
                return True
            else:
                # Frame changed or periodic full check - run full detection
                if force_full_check:
                    Log.debug("Periodic full detection check...")
                else:
                    Log.debug("Quick check indicates signal may have returned, running full detection...")
                self._last_no_signal_result = self.detector.is_no_signal(
                    frame, debug=True
                )
                if not self._last_no_signal_result:
                    Log.info("Signal restored - content detected")
                else:
                    Log.debug("Full detection still shows no signal")
        else:
            # Currently has signal - quick check if frame became dark
            has_content = self.detector.quick_check_has_content(frame)
            if has_content:
                # Still has colorful content, no need for full check
                return False
            else:
                # Frame is mostly dark - run full detection to confirm no-signal
                self._last_no_signal_result = self.detector.is_no_signal(
                    frame, debug=True
                )
                if self._last_no_signal_result:
                    Log.info("No signal detected - Elgato logo found")

        return self._last_no_signal_result

    def read_frame(self) -> tuple[QPixmap | None, bool]:
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
        """Release the camera capture."""
        if self.cap and self.cap.isOpened():
            self.cap.release()

    def toggle_signal(self) -> bool:
        """Toggle signal state (test mode only). Returns new signal state."""
        if self.test_mode and hasattr(self.cap, "toggle_signal"):
            return self.cap.toggle_signal()
        return True
