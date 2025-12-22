"""
No-signal detection using computer vision.

Uses multi-scale template matching to find the Elgato logo and validates
that the surrounding background is grey/dark.
"""

import os

import cv2
import numpy as np

from hdmi_viewer.log import Log
from hdmi_viewer.utils import get_resource_path


class NoSignalDetector:
    """
    Detect Elgato 'No Signal' screen using template matching.

    Approach:
    1. Multi-scale template matching to find the logo at any size
    2. Validate that background pixels are grey/dark
    """

    # Template matching threshold (0-1, higher = stricter match)
    MATCH_THRESHOLD = 0.7

    # Background validation settings
    GREY_TOLERANCE = 30  # Max deviation from grey (where R≈G≈B)
    DARK_THRESHOLD = 80  # Max brightness for "dark" pixels
    BACKGROUND_GREY_RATIO = 0.85  # % of background that must be dark grey

    # Scale range for multi-scale matching
    SCALE_MIN = 0.1
    SCALE_MAX = 0.8
    SCALE_STEPS = 15

    def __init__(self):
        self.logo_template = None
        self.logo_template_gray = None
        self._load_template()

    def _load_template(self):
        """Load and extract the logo template from the reference image."""
        ref_path = get_resource_path("elgato_no_source.png")

        if not os.path.exists(ref_path):
            Log.warning(f"NoSignalDetector: Reference image not found at {ref_path}")
            return

        ref_img = cv2.imread(ref_path)
        if ref_img is None:
            Log.warning("NoSignalDetector: Failed to read reference image")
            return

        # Extract the logo from the center of the reference image
        # The Elgato logo is typically in the center ~40% of the image
        h, w = ref_img.shape[:2]
        margin_x = int(w * 0.3)
        margin_y = int(h * 0.3)

        self.logo_template = ref_img[margin_y:h-margin_y, margin_x:w-margin_x].copy()
        self.logo_template_gray = cv2.cvtColor(self.logo_template, cv2.COLOR_BGR2GRAY)

        Log.success(
            f"NoSignalDetector: Loaded logo template {self.logo_template.shape[1]}x{self.logo_template.shape[0]}"
        )

    def _multi_scale_template_match(self, frame_gray, template_gray):
        """
        Find template in frame at multiple scales.
        Returns (best_score, best_location, best_scale, best_size) or (0, None, None, None) if not found.
        """
        frame_h, frame_w = frame_gray.shape[:2]
        templ_h, templ_w = template_gray.shape[:2]

        best_score = 0
        best_loc = None
        best_scale = None
        best_size = None

        # Generate scales to test
        scales = np.linspace(self.SCALE_MIN, self.SCALE_MAX, self.SCALE_STEPS)

        for scale in scales:
            # Calculate new template size relative to frame
            new_w = int(frame_w * scale)
            new_h = int(new_w * templ_h / templ_w)  # Maintain aspect ratio

            if new_w < 20 or new_h < 20:
                continue
            if new_w >= frame_w or new_h >= frame_h:
                continue

            # Resize template
            scaled_template = cv2.resize(template_gray, (new_w, new_h))

            # Template matching
            result = cv2.matchTemplate(frame_gray, scaled_template, cv2.TM_CCOEFF_NORMED)
            _, max_val, _, max_loc = cv2.minMaxLoc(result)

            if max_val > best_score:
                best_score = max_val
                best_loc = max_loc
                best_scale = scale
                best_size = (new_w, new_h)

        return best_score, best_loc, best_scale, best_size

    def _is_background_grey(self, frame, logo_rect=None):
        """
        Check if the frame background (excluding logo area) is dark grey.

        Args:
            frame: BGR image
            logo_rect: (x, y, w, h) of detected logo to exclude, or None

        Returns:
            (is_grey, ratio) - whether background is grey and what percentage
        """
        h, w = frame.shape[:2]

        # Create mask for background (everything except logo)
        mask = np.ones((h, w), dtype=np.uint8) * 255

        if logo_rect is not None:
            x, y, lw, lh = logo_rect
            # Add some padding around logo
            pad = 10
            x1 = max(0, x - pad)
            y1 = max(0, y - pad)
            x2 = min(w, x + lw + pad)
            y2 = min(h, y + lh + pad)
            mask[y1:y2, x1:x2] = 0

        # Get background pixels
        bg_pixels = frame[mask > 0]

        if len(bg_pixels) == 0:
            return False, 0.0

        # Check if pixels are dark and grey (R ≈ G ≈ B, all low)
        b, g, r = bg_pixels[:, 0], bg_pixels[:, 1], bg_pixels[:, 2]

        # Calculate "greyness" - how close R, G, B are to each other
        max_channel = np.maximum(np.maximum(r, g), b)
        min_channel = np.minimum(np.minimum(r, g), b)
        color_spread = max_channel - min_channel

        # Pixel is "dark grey" if:
        # - Low brightness (max channel < threshold)
        # - Low color spread (R, G, B are similar)
        is_dark = max_channel < self.DARK_THRESHOLD
        is_grey = color_spread < self.GREY_TOLERANCE
        is_dark_grey = is_dark & is_grey

        ratio = np.sum(is_dark_grey) / len(bg_pixels)

        return ratio >= self.BACKGROUND_GREY_RATIO, ratio

    def is_no_signal(self, frame) -> bool:
        """
        Detect if frame is the Elgato no-signal screen.

        Returns True if:
        1. The Elgato logo is found via template matching
        2. The background is predominantly dark grey
        """
        if self.logo_template is None:
            return False

        frame_gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)

        # Step 1: Find the logo using multi-scale template matching
        score, loc, scale, size = self._multi_scale_template_match(
            frame_gray, self.logo_template_gray
        )

        if score < self.MATCH_THRESHOLD or loc is None:
            return False

        # Step 2: Validate background is dark grey
        logo_rect = (loc[0], loc[1], size[0], size[1]) if size else None
        is_grey, grey_ratio = self._is_background_grey(frame, logo_rect)

        return is_grey

    def get_detection_details(self, frame) -> dict:
        """
        Get detailed detection info for debugging.
        Returns dict with match_score, logo_location, background_grey_ratio, etc.
        """
        if self.logo_template is None:
            return {"error": "Template not loaded"}

        frame_gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)

        score, loc, scale, size = self._multi_scale_template_match(
            frame_gray, self.logo_template_gray
        )

        logo_rect = (loc[0], loc[1], size[0], size[1]) if loc and size else None
        is_grey, grey_ratio = self._is_background_grey(frame, logo_rect)

        return {
            "match_score": score,
            "match_threshold": self.MATCH_THRESHOLD,
            "logo_found": score >= self.MATCH_THRESHOLD,
            "logo_location": loc,
            "logo_size": size,
            "logo_scale": scale,
            "background_grey_ratio": grey_ratio,
            "background_threshold": self.BACKGROUND_GREY_RATIO,
            "background_valid": is_grey,
            "is_no_signal": score >= self.MATCH_THRESHOLD and is_grey,
        }


# Shared detector instance (loaded once, used by all feeds)
_no_signal_detector: NoSignalDetector | None = None


def get_no_signal_detector() -> NoSignalDetector:
    """Get or create the shared NoSignalDetector instance."""
    global _no_signal_detector
    if _no_signal_detector is None:
        _no_signal_detector = NoSignalDetector()
    return _no_signal_detector
