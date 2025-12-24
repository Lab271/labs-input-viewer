"""
Threaded camera worker for non-blocking frame capture.

Uses QThread to read frames from cameras without blocking the UI.
"""

from PyQt6.QtCore import QThread, pyqtSignal

from hdmi_viewer.camera import CameraFeed
from hdmi_viewer.log import Log


class CameraWorker(QThread):
    """Worker thread that continuously reads frames from a camera feed."""

    # Emitted when a new frame is ready: (pixmap, has_signal)
    frame_ready = pyqtSignal(object, bool)

    # Emitted when camera is switched
    camera_switched = pyqtSignal(str)  # input name

    def __init__(
        self,
        camera_index: int,
        test_mode: bool = False,
        label: str = "FEED",
        switch_signals: bool = False,
        always_no_signal: bool = False,
        parent=None,
    ):
        super().__init__(parent)
        self._running = False
        self._paused = False

        # Camera settings
        self.camera_index = camera_index
        self.test_mode = test_mode
        self.label = label
        self.switch_signals = switch_signals
        self.always_no_signal = always_no_signal

        # Camera feed (created in run() to ensure it's in the worker thread)
        self.feed: CameraFeed | None = None

        # Switch request (set from main thread, processed in worker thread)
        self._switch_to_index: int | None = None

    def run(self):
        """Main worker loop - continuously read frames."""
        self._running = True
        self._create_feed()

        while self._running:
            # Check for camera switch request
            if self._switch_to_index is not None:
                self._do_switch()

            # Skip if paused
            if self._paused:
                self.msleep(50)
                continue

            # Read frame
            if self.feed:
                pixmap, has_signal = self.feed.read_frame()
                self.frame_ready.emit(pixmap, has_signal)

            # Small sleep to prevent CPU spinning (target ~30fps = 33ms)
            self.msleep(16)  # ~60fps capture, UI will display at its own rate

        # Cleanup
        if self.feed:
            self.feed.release()
            self.feed = None

    def _create_feed(self):
        """Create the camera feed."""
        self.feed = CameraFeed(
            self.camera_index,
            self.test_mode,
            self.label,
            self.switch_signals,
            self.always_no_signal,
        )

    def _do_switch(self):
        """Switch to a new camera (called from worker thread)."""
        new_index = self._switch_to_index
        self._switch_to_index = None

        if self.feed:
            self.feed.release()

        self.camera_index = new_index
        self._create_feed()
        Log.info(f"{self.label} worker switched to input {new_index}")

    def switch_camera(self, new_index: int):
        """Request camera switch (called from main thread, thread-safe)."""
        self._switch_to_index = new_index

    def pause(self):
        """Pause frame capture."""
        self._paused = True

    def resume(self):
        """Resume frame capture."""
        self._paused = False

    def stop(self):
        """Stop the worker thread."""
        self._running = False
        self.wait(2000)  # Wait up to 2 seconds for thread to finish

    def toggle_signal(self) -> bool:
        """Toggle signal state (test mode only)."""
        if self.feed:
            return self.feed.toggle_signal()
        return True
