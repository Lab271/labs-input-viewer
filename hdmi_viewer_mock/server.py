"""
Virtual camera server using pyvirtualcam.
"""

import signal
import sys
import time

import cv2
import numpy as np

from .config import PATTERN_MAP, PatternType
from .patterns import PatternGenerator


class VirtualCameraServer:
    """
    Streams test patterns to a virtual camera device.
    
    Uses pyvirtualcam to create a virtual webcam that can be
    captured by the main HDMI Viewer application.
    """

    def __init__(
        self,
        width: int = 1920,
        height: int = 1080,
        fps: int = 30,
        verbose: bool = False,
    ):
        self.width = width
        self.height = height
        self.fps = fps
        self.verbose = verbose

        self.pattern = PatternType.BARS
        self.no_signal = False
        self.running = False

        self.generator = PatternGenerator(width, height, fps)
        self.camera = None

        # Callback for UI updates
        self.on_status_change = None

    def log(self, message: str):
        """Print log message if verbose mode is enabled."""
        if self.verbose:
            print(f"[MockServer] {message}")

    def set_pattern(self, pattern: PatternType):
        """Change the current test pattern."""
        self.pattern = pattern
        self.log(f"Pattern changed to: {pattern.name}")

    def set_no_signal(self, enabled: bool):
        """Toggle no-signal mode."""
        self.no_signal = enabled
        self.log(f"No-signal mode: {enabled}")

    def toggle_no_signal(self):
        """Toggle no-signal mode and return new state."""
        self.no_signal = not self.no_signal
        self.log(f"No-signal toggled to: {self.no_signal}")
        return self.no_signal

    def start(self):
        """Start the virtual camera server."""
        import pyvirtualcam

        self.log(f"Starting virtual camera: {self.width}x{self.height} @ {self.fps}fps")

        try:
            # Create virtual camera
            self.camera = pyvirtualcam.Camera(
                width=self.width,
                height=self.height,
                fps=self.fps,
            )
            self.log(f"Virtual camera created: {self.camera.device}")

            self.running = True
            self._stream_loop()

        except Exception as e:
            print(f"Error starting virtual camera: {e}")
            raise
        finally:
            if self.camera:
                self.camera.close()
                self.camera = None

    def stop(self):
        """Stop the virtual camera server."""
        self.running = False
        self.log("Stopping virtual camera server")

    def _stream_loop(self):
        """Main streaming loop."""
        frame_time = 1.0 / self.fps

        while self.running:
            start = time.time()

            # Generate frame
            if self.no_signal:
                frame = self.generator.generate_no_signal()
            else:
                frame = self.generator.generate(self.pattern)

            # pyvirtualcam expects RGB, OpenCV uses BGR
            frame_rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)

            # Send to virtual camera
            self.camera.send(frame_rgb)

            # Maintain frame rate
            elapsed = time.time() - start
            sleep_time = frame_time - elapsed
            if sleep_time > 0:
                time.sleep(sleep_time)

    def run_single_frame(self) -> np.ndarray | None:
        """Generate a single frame without streaming (for preview)."""
        if self.no_signal:
            return self.generator.generate_no_signal()
        return self.generator.generate(self.pattern)


def run_mock_server(
    headless: bool = False,
    pattern: str = "bars",
    no_signal: bool = False,
    width: int = 1920,
    height: int = 1080,
    fps: int = 30,
    verbose: bool = False,
):
    """
    Main entry point for the mock server.
    
    Args:
        headless: Run without GUI
        pattern: Initial pattern type
        no_signal: Start in no-signal mode
        width: Output width
        height: Output height
        fps: Output frame rate
        verbose: Enable verbose logging
    """
    # Convert pattern string to enum
    pattern_type = PATTERN_MAP.get(pattern, PatternType.BARS)

    server = VirtualCameraServer(
        width=width,
        height=height,
        fps=fps,
        verbose=verbose,
    )
    server.pattern = pattern_type
    server.no_signal = no_signal

    # Handle Ctrl+C gracefully
    def signal_handler(sig, frame):
        print("\nShutting down...")
        server.stop()
        sys.exit(0)

    signal.signal(signal.SIGINT, signal_handler)

    if headless:
        # Run without GUI
        print("Starting mock server in headless mode")
        print(f"Resolution: {width}x{height} @ {fps}fps")
        print(f"Pattern: {pattern}")
        print(f"No-signal: {no_signal}")
        print("Press Ctrl+C to stop")
        server.start()
    else:
        # Run with GUI control panel
        from .ui import run_with_gui
        run_with_gui(server)
