"""
Test pattern generators for the mock video server.
"""

import os
import time

import cv2
import numpy as np

from .config import PatternType


class PatternGenerator:
    """
    Generates various test patterns for the virtual camera.
    """

    # Color palette for patterns
    COLORS = [
        (255, 0, 0),      # Red
        (0, 255, 0),      # Green
        (0, 0, 255),      # Blue
        (255, 255, 0),    # Yellow
        (255, 0, 255),    # Magenta
        (0, 255, 255),    # Cyan
        (255, 255, 255),  # White
    ]

    def __init__(self, width: int, height: int, fps: int = 30):
        self.width = width
        self.height = height
        self.fps = fps
        self.start_time = time.time()
        self.frame_count = 0

        # Load assets
        self._load_assets()

    def _load_assets(self):
        """Load image assets for no-signal frames."""
        script_dir = os.path.dirname(os.path.abspath(__file__))
        assets_dir = os.path.join(script_dir, "assets")

        # Elgato no-signal image
        self.elgato_img = None
        elgato_path = os.path.join(assets_dir, "elgato_no_source.png")
        if os.path.exists(elgato_path):
            self.elgato_img = cv2.imread(elgato_path)

        # Zed image for no-signal overlay
        self.zed_img = None
        zed_path = os.path.join(assets_dir, "zed.png")
        if os.path.exists(zed_path):
            self.zed_img = cv2.imread(zed_path, cv2.IMREAD_UNCHANGED)

    @property
    def elapsed(self) -> float:
        """Seconds since generator started."""
        return time.time() - self.start_time

    def generate(self, pattern: PatternType) -> np.ndarray:
        """
        Generate a frame for the specified pattern.

        Args:
            pattern: The pattern type to generate

        Returns:
            BGR frame as numpy array
        """
        self.frame_count += 1

        if pattern == PatternType.BARS:
            return self._generate_bars()
        elif pattern == PatternType.GRADIENT:
            return self._generate_gradient()
        elif pattern == PatternType.BOUNCING:
            return self._generate_bouncing()
        elif pattern == PatternType.STATIC:
            return self._generate_static()
        else:
            return self._generate_bars()

    def generate_no_signal(self) -> np.ndarray:
        """
        Generate a no-signal frame.
        Cycles through: Elgato screenshot, zed image, grey screen.
        """
        w, h = self.width, self.height
        cycle_phase = int(self.elapsed / 2) % 3

        if cycle_phase == 0 and self.elgato_img is not None:
            # Elgato no-signal screenshot
            return cv2.resize(self.elgato_img, (w, h))

        elif cycle_phase == 1 and self.zed_img is not None:
            # Zed centered on grey
            frame = np.full((h, w, 3), 45, dtype=np.uint8)
            self._overlay_image(frame, self.zed_img)
            return frame

        else:
            # Plain grey
            return np.full((h, w, 3), 45, dtype=np.uint8)

    def _overlay_image(self, frame: np.ndarray, image: np.ndarray):
        """Overlay an image (with optional alpha) centered on frame."""
        h, w = frame.shape[:2]
        img_h, img_w = image.shape[:2]

        x_offset = max(0, (w - img_w) // 2)
        y_offset = max(0, (h - img_h) // 2)
        x_end = min(x_offset + img_w, w)
        y_end = min(y_offset + img_h, h)

        actual_w = x_end - x_offset
        actual_h = y_end - y_offset

        if image.shape[2] == 4:
            # Has alpha channel
            rgb = image[:actual_h, :actual_w, :3]
            alpha = image[:actual_h, :actual_w, 3:4] / 255.0

            roi = frame[y_offset:y_end, x_offset:x_end]
            frame[y_offset:y_end, x_offset:x_end] = (
                alpha * rgb + (1 - alpha) * roi
            ).astype(np.uint8)
        else:
            frame[y_offset:y_end, x_offset:x_end] = image[:actual_h, :actual_w]

    def _generate_bars(self) -> np.ndarray:
        """Generate animated color bars pattern."""
        w, h = self.width, self.height
        frame = np.zeros((h, w, 3), dtype=np.uint8)

        # Animated gradient background
        phase = (self.elapsed * 0.5) % 1.0
        for y in range(h):
            intensity = int(30 + 20 * np.sin(2 * np.pi * (y / h + phase)))
            frame[y, :] = [intensity, intensity, intensity + 10]

        # Moving color bars
        bar_width = w // len(self.COLORS)
        offset = int((self.elapsed * 100) % bar_width)

        bar_height = h // 3
        bar_top = h // 3

        for i, color in enumerate(self.COLORS):
            x1 = (i * bar_width + offset) % w
            x2 = x1 + bar_width - 10

            if x2 > w:
                cv2.rectangle(frame, (x1, bar_top), (w, bar_top + bar_height), color, -1)
                cv2.rectangle(frame, (0, bar_top), (x2 - w, bar_top + bar_height), color, -1)
            else:
                cv2.rectangle(frame, (x1, bar_top), (x2, bar_top + bar_height), color, -1)

        self._add_overlay_text(frame)
        return frame

    def _generate_gradient(self) -> np.ndarray:
        """Generate animated gradient pattern."""
        w, h = self.width, self.height

        # Create animated gradient
        phase = self.elapsed * 0.5

        # Horizontal gradient
        x_grad = np.linspace(0, 1, w)
        y_grad = np.linspace(0, 1, h)

        # Create RGB channels with different phase offsets
        r = (np.sin(2 * np.pi * (x_grad + phase)) * 127 + 128).astype(np.uint8)
        g = (np.sin(2 * np.pi * (y_grad[:, np.newaxis] + phase * 0.7)) * 127 + 128).astype(np.uint8)
        b = (np.sin(2 * np.pi * (x_grad + y_grad[:, np.newaxis] + phase * 1.3)) * 127 + 128).astype(np.uint8)

        frame = np.zeros((h, w, 3), dtype=np.uint8)
        frame[:, :, 0] = b
        frame[:, :, 1] = np.broadcast_to(g, (h, w))
        frame[:, :, 2] = np.broadcast_to(r, (h, w))

        self._add_overlay_text(frame)
        return frame

    def _generate_bouncing(self) -> np.ndarray:
        """Generate bouncing shapes pattern."""
        w, h = self.width, self.height
        frame = np.full((h, w, 3), 30, dtype=np.uint8)

        # Multiple bouncing circles
        for i in range(5):
            freq_x = 1.5 + i * 0.3
            freq_y = 2.0 + i * 0.2
            radius = 30 + i * 15

            cx = int(w / 2 + (w / 3 - radius) * np.sin(self.elapsed * freq_x + i))
            cy = int(h / 2 + (h / 3 - radius) * np.cos(self.elapsed * freq_y + i * 0.5))

            color = self.COLORS[i % len(self.COLORS)]
            cv2.circle(frame, (cx, cy), radius, color, -1)
            cv2.circle(frame, (cx, cy), radius, (255, 255, 255), 2)

        self._add_overlay_text(frame)
        return frame

    def _generate_static(self) -> np.ndarray:
        """Generate static noise pattern."""
        w, h = self.width, self.height

        # Random noise
        noise = np.random.randint(0, 256, (h, w, 3), dtype=np.uint8)

        # Add some structure with horizontal lines
        for y in range(0, h, 8):
            intensity = np.random.randint(50, 200)
            noise[y:y+2, :] = intensity

        self._add_overlay_text(noise)
        return noise

    def _add_overlay_text(self, frame: np.ndarray):
        """Add overlay text to frame."""
        w, h = self.width, self.height

        # Mock label
        cv2.putText(
            frame, "MOCK CAMERA",
            (50, 80), cv2.FONT_HERSHEY_SIMPLEX,
            2.0, (255, 255, 255), 3
        )

        # Resolution and frame info
        info = f"{w}x{h} @ {self.fps}fps | Frame: {self.frame_count}"
        cv2.putText(
            frame, info,
            (50, h - 50), cv2.FONT_HERSHEY_SIMPLEX,
            0.8, (200, 200, 200), 2
        )

        # Timestamp
        cv2.putText(
            frame, f"Time: {self.elapsed:.1f}s",
            (w - 280, h - 50), cv2.FONT_HERSHEY_SIMPLEX,
            0.8, (200, 200, 200), 2
        )

        # TEST MODE indicator
        cv2.putText(
            frame, "TEST MODE",
            (w - 260, 80), cv2.FONT_HERSHEY_SIMPLEX,
            1.0, (0, 165, 255), 3
        )

        # Border
        cv2.rectangle(frame, (5, 5), (w - 5, h - 5), (100, 100, 100), 2)
