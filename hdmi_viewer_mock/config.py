"""
Configuration for the mock video server.
"""

from dataclasses import dataclass
from enum import Enum, auto


class PatternType(Enum):
    """Available test pattern types."""
    BARS = auto()       # Color bars
    GRADIENT = auto()   # Animated gradient
    BOUNCING = auto()   # Bouncing shapes
    STATIC = auto()     # Static noise


@dataclass
class MockConfig:
    """Configuration for the mock video server."""

    width: int = 1920
    height: int = 1080
    fps: int = 30
    pattern: PatternType = PatternType.BARS
    no_signal: bool = False
    label: str = "MOCK"

    # Signal cycling
    switch_signals: bool = False
    signal_duration: float = 10.0  # Seconds before switching


# Pattern string to enum mapping
PATTERN_MAP = {
    "bars": PatternType.BARS,
    "gradient": PatternType.GRADIENT,
    "bouncing": PatternType.BOUNCING,
    "static": PatternType.STATIC,
}
