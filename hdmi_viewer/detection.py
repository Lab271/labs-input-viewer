"""
No-signal detection using computer vision.

Uses multi-vector feature extraction and comparison against a reference image
to detect the Elgato 'No Signal' screen.
"""

import os

import cv2
import numpy as np

from hdmi_viewer.log import Log
from hdmi_viewer.utils import get_resource_path


class NoSignalDetector:
    """
    Simple vision model to detect Elgato 'No Signal' screen.
    Uses multi-vector feature extraction and comparison against reference image.
    """

    # Feature vector size for comparison
    FEATURE_SIZE = 64
    # Similarity threshold (0-1, higher = more similar)
    SIMILARITY_THRESHOLD = 0.80

    def __init__(self):
        self.reference_features = None
        self._load_reference_image()

    def _load_reference_image(self):
        """Load and extract features from the reference no-signal image."""
        ref_path = get_resource_path("elgato_no_source.png")

        if os.path.exists(ref_path):
            ref_img = cv2.imread(ref_path)
            if ref_img is not None:
                self.reference_features = self._extract_features(ref_img)
                Log.success(
                    f"NoSignalDetector: Loaded reference image, feature dim={len(self.reference_features)}"
                )
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
_no_signal_detector: NoSignalDetector | None = None


def get_no_signal_detector() -> NoSignalDetector:
    """Get or create the shared NoSignalDetector instance."""
    global _no_signal_detector
    if _no_signal_detector is None:
        _no_signal_detector = NoSignalDetector()
    return _no_signal_detector
