"""
Mock video sources for testing without hardware.

Provides animated test patterns and simulated no-signal states.
"""

import cv2
import numpy as np
import time
import os
from dataclasses import dataclass


@dataclass
class MockConfig:
    """Configuration for a mock video source."""
    width: int = 1920
    height: int = 1080
    fps: int = 30
    label: str = "FEED"
    switch_signals: bool = False    # Cycle between signal and no-signal
    always_no_signal: bool = False  # Always show no-signal
    signal_duration: float = 10.0   # Seconds of signal before switching


class MockVideoSource:
    """
    Generates animated test patterns for testing without capture hardware.
    
    Modes:
        - Normal: Always shows animated test pattern
        - switch_signals: Cycles between signal (10s) and no-signal (10s)
        - always_no_signal: Always shows no-signal (Elgato screenshot)
    """
    
    def __init__(self, config: MockConfig, source_id: int = 0):
        self.config = config
        self.source_id = source_id
        self.start_time = time.time()
        self.frame_count = 0
        self.no_signal = False
        
        # Load the Elgato no-signal screenshot if available
        self.elgato_no_signal_img = None
        script_dir = os.path.dirname(os.path.abspath(__file__))
        no_signal_path = os.path.join(script_dir, "elgato_no_source.png")
        if os.path.exists(no_signal_path):
            self.elgato_no_signal_img = cv2.imread(no_signal_path)
            print(f"Loaded Elgato no-signal image: {no_signal_path}")
        
        # Colors for the animated pattern
        self.colors = [
            (255, 0, 0),      # Red
            (0, 255, 0),      # Green
            (0, 0, 255),      # Blue
            (255, 255, 0),    # Yellow
            (255, 0, 255),    # Magenta
            (0, 255, 255),    # Cyan
            (255, 255, 255),  # White
        ]
    
    def isOpened(self) -> bool:
        return True
    
    def get(self, prop_id: int):
        """Mimic cv2.VideoCapture.get()"""
        if prop_id == cv2.CAP_PROP_FRAME_WIDTH:
            return self.config.width
        elif prop_id == cv2.CAP_PROP_FRAME_HEIGHT:
            return self.config.height
        elif prop_id == cv2.CAP_PROP_FPS:
            return self.config.fps
        return 0
    
    def set(self, prop_id: int, value) -> bool:
        """Mimic cv2.VideoCapture.set() - ignored for mock"""
        return True
    
    def read(self):
        """
        Generate and return an animated test frame.
        Returns (success, frame) like cv2.VideoCapture.read()
        """
        # In always_no_signal mode, always show no-signal
        if self.config.always_no_signal:
            return True, self._generate_no_signal_frame()
        
        # In switch_signals mode, cycle between signal and no-signal
        if self.config.switch_signals:
            elapsed = time.time() - self.start_time
            cycle_time = self.config.signal_duration * 2  # Full cycle
            in_signal_phase = (elapsed % cycle_time) < self.config.signal_duration
            
            if not in_signal_phase:
                return True, self._generate_no_signal_frame()
        elif self.no_signal:
            return True, self._generate_no_signal_frame()
        
        frame = self._generate_animated_frame()
        self.frame_count += 1
        return True, frame
    
    def release(self):
        """Mimic cv2.VideoCapture.release()"""
        pass
    
    def toggle_signal(self):
        """Toggle between signal and no-signal state."""
        self.no_signal = not self.no_signal
        return not self.no_signal  # Return current signal state
    
    def _generate_animated_frame(self) -> np.ndarray:
        """Generate an animated test pattern frame."""
        w, h = self.config.width, self.config.height
        frame = np.zeros((h, w, 3), dtype=np.uint8)
        
        elapsed = time.time() - self.start_time
        
        # Animated gradient background
        phase = (elapsed * 0.5) % 1.0
        for y in range(h):
            intensity = int(30 + 20 * np.sin(2 * np.pi * (y / h + phase)))
            frame[y, :] = [intensity, intensity, intensity + 10]
        
        # Moving color bars
        bar_width = w // len(self.colors)
        offset = int((elapsed * 100) % bar_width)
        
        bar_height = h // 3
        bar_top = h // 3
        
        for i, color in enumerate(self.colors):
            x1 = (i * bar_width + offset) % w
            x2 = x1 + bar_width - 10
            if x2 > w:
                # Wrap around
                cv2.rectangle(frame, (x1, bar_top), (w, bar_top + bar_height), color, -1)
                cv2.rectangle(frame, (0, bar_top), (x2 - w, bar_top + bar_height), color, -1)
            else:
                cv2.rectangle(frame, (x1, bar_top), (x2, bar_top + bar_height), color, -1)
        
        # Bouncing circle
        circle_x = int(w/2 + (w/3) * np.sin(elapsed * 2))
        circle_y = int(h/4 + (h/8) * np.cos(elapsed * 3))
        cv2.circle(frame, (circle_x, circle_y), 40, (0, 255, 255), -1)
        cv2.circle(frame, (circle_x, circle_y), 40, (255, 255, 255), 3)
        
        # Feed label and info
        label = f"{self.config.label} {self.source_id + 1}"
        cv2.putText(frame, label, (50, 80), 
                    cv2.FONT_HERSHEY_SIMPLEX, 2.5, (255, 255, 255), 4)
        
        # Resolution and frame info
        info = f"{w}x{h} @ {self.config.fps}fps | Frame: {self.frame_count}"
        cv2.putText(frame, info, (50, h - 50),
                    cv2.FONT_HERSHEY_SIMPLEX, 1, (200, 200, 200), 2)
        
        # Timestamp
        timestamp = f"Time: {elapsed:.1f}s"
        cv2.putText(frame, timestamp, (w - 300, h - 50),
                    cv2.FONT_HERSHEY_SIMPLEX, 1, (200, 200, 200), 2)
        
        # TEST MODE indicator
        cv2.putText(frame, "TEST MODE", (w - 280, 80),
                    cv2.FONT_HERSHEY_SIMPLEX, 1.2, (0, 165, 255), 3)
        
        # Border
        cv2.rectangle(frame, (5, 5), (w-5, h-5), (100, 100, 100), 2)
        
        return frame
    
    def _generate_no_signal_frame(self) -> np.ndarray:
        """Generate a frame that mimics Elgato's no-signal screen."""
        w, h = self.config.width, self.config.height
        
        # Use actual Elgato screenshot if available
        if self.elgato_no_signal_img is not None:
            # Resize to match expected dimensions
            return cv2.resize(self.elgato_no_signal_img, (w, h))
        
        # Fallback: Dark gray background (matches Elgato no-signal)
        frame = np.full((h, w, 3), 45, dtype=np.uint8)
        
        # Add slight noise to make it more realistic
        noise = np.random.randint(-3, 3, (h, w, 3), dtype=np.int16)
        frame = np.clip(frame.astype(np.int16) + noise, 0, 255).astype(np.uint8)
        
        return frame


def create_mock_feed(camera_index: int, target_width: int, target_height: int, 
                     target_fps: int, label: str = "FEED", 
                     switch_signals: bool = False,
                     always_no_signal: bool = False) -> MockVideoSource:
    """
    Factory function to create a mock video source.
    Matches the signature expected by the main application.
    
    Args:
        switch_signals: If True, cycles between signal (10s) and no-signal (10s)
        always_no_signal: If True, always shows no-signal state
    """
    config = MockConfig(
        width=target_width,
        height=target_height,
        fps=target_fps,
        label=f"{label} (CAM {camera_index})",
        switch_signals=switch_signals,
        always_no_signal=always_no_signal
    )
    return MockVideoSource(config, source_id=camera_index)
