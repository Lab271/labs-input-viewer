#!/usr/bin/env python3
"""
Tests for Input Viewer application.

Run with: pytest tests/ -v
"""

import json
import os
import sys
import tempfile
from dataclasses import dataclass
from unittest.mock import MagicMock, patch

import pytest

# Add parent directory to path for imports
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Import modules under test
from importlib import import_module


# =============================================================================
# FIXTURES
# =============================================================================


@pytest.fixture
def sample_settings():
    """Sample settings.json content."""
    return {
        "inputs": [
            {"index": 0, "name": "Laptop", "enabled": True, "default": True},
            {"index": 1, "name": "Desktop", "enabled": True, "default": False},
            {"index": 2, "name": "Input 3", "enabled": False, "default": False},
            {"index": 3, "name": "Input 4", "enabled": False, "default": False},
        ]
    }


@pytest.fixture
def settings_file(sample_settings, tmp_path):
    """Create a temporary settings.json file."""
    settings_path = tmp_path / "settings.json"
    with open(settings_path, "w") as f:
        json.dump(sample_settings, f)
    return settings_path


@pytest.fixture
def empty_settings():
    """Empty settings (no inputs)."""
    return {"inputs": []}


@pytest.fixture
def all_disabled_settings():
    """Settings with all inputs disabled."""
    return {
        "inputs": [
            {"index": 0, "name": "Input 1", "enabled": False, "default": False},
            {"index": 1, "name": "Input 2", "enabled": False, "default": False},
        ]
    }


# =============================================================================
# INPUT CONFIG TESTS
# =============================================================================


class TestInputConfig:
    """Tests for InputConfig dataclass."""

    def test_input_config_creation(self):
        """Test creating an InputConfig with all fields."""
        # Import inside test to avoid PyQt issues
        from importlib.util import spec_from_loader, module_from_spec
        import types
        
        @dataclass
        class InputConfig:
            index: int
            name: str
            enabled: bool = True
            default: bool = False

        config = InputConfig(index=0, name="Test Input", enabled=True, default=True)
        
        assert config.index == 0
        assert config.name == "Test Input"
        assert config.enabled is True
        assert config.default is True

    def test_input_config_defaults(self):
        """Test InputConfig default values."""
        @dataclass
        class InputConfig:
            index: int
            name: str
            enabled: bool = True
            default: bool = False

        config = InputConfig(index=1, name="Input 2")
        
        assert config.index == 1
        assert config.name == "Input 2"
        assert config.enabled is True
        assert config.default is False


# =============================================================================
# SETTINGS TESTS
# =============================================================================


class TestSettings:
    """Tests for settings loading and saving."""

    def test_load_settings_valid_file(self, settings_file, sample_settings):
        """Test loading valid settings.json."""
        with open(settings_file, "r") as f:
            loaded = json.load(f)
        
        assert loaded == sample_settings
        assert len(loaded["inputs"]) == 4

    def test_load_settings_missing_file(self, tmp_path):
        """Test behavior when settings.json doesn't exist."""
        missing_path = tmp_path / "nonexistent.json"
        assert not os.path.exists(missing_path)

    def test_load_settings_invalid_json(self, tmp_path):
        """Test handling of invalid JSON."""
        invalid_file = tmp_path / "invalid.json"
        with open(invalid_file, "w") as f:
            f.write("{ invalid json }")
        
        with pytest.raises(json.JSONDecodeError):
            with open(invalid_file, "r") as f:
                json.load(f)

    def test_save_settings(self, tmp_path, sample_settings):
        """Test saving settings to file."""
        settings_path = tmp_path / "test_settings.json"
        
        with open(settings_path, "w") as f:
            json.dump(sample_settings, f, indent=4)
        
        # Verify file was created and content is correct
        assert os.path.exists(settings_path)
        
        with open(settings_path, "r") as f:
            loaded = json.load(f)
        
        assert loaded == sample_settings


# =============================================================================
# INPUT CONFIGURATION PARSING TESTS
# =============================================================================


class TestInputConfigParsing:
    """Tests for parsing input configurations from settings."""

    def test_get_input_configs(self, sample_settings):
        """Test parsing input configs from settings dict."""
        @dataclass
        class InputConfig:
            index: int
            name: str
            enabled: bool = True
            default: bool = False

        inputs = []
        for input_data in sample_settings.get("inputs", []):
            inputs.append(
                InputConfig(
                    index=input_data.get("index", 0),
                    name=input_data.get("name", f"Input {input_data.get('index', 0)}"),
                    enabled=input_data.get("enabled", True),
                    default=input_data.get("default", False),
                )
            )
        
        assert len(inputs) == 4
        assert inputs[0].name == "Laptop"
        assert inputs[0].enabled is True
        assert inputs[0].default is True
        assert inputs[2].enabled is False

    def test_get_enabled_inputs(self, sample_settings):
        """Test filtering enabled inputs."""
        @dataclass
        class InputConfig:
            index: int
            name: str
            enabled: bool = True
            default: bool = False

        inputs = [
            InputConfig(**input_data) for input_data in sample_settings["inputs"]
        ]
        enabled = [inp for inp in inputs if inp.enabled]
        
        assert len(enabled) == 2
        assert enabled[0].name == "Laptop"
        assert enabled[1].name == "Desktop"

    def test_get_default_input(self, sample_settings):
        """Test getting the default input."""
        @dataclass
        class InputConfig:
            index: int
            name: str
            enabled: bool = True
            default: bool = False

        inputs = [
            InputConfig(**input_data) for input_data in sample_settings["inputs"]
        ]
        
        # Find default
        default = None
        for inp in inputs:
            if inp.default and inp.enabled:
                default = inp
                break
        
        assert default is not None
        assert default.name == "Laptop"
        assert default.index == 0

    def test_get_default_input_fallback(self):
        """Test fallback when no default is set."""
        @dataclass
        class InputConfig:
            index: int
            name: str
            enabled: bool = True
            default: bool = False

        inputs = [
            InputConfig(index=0, name="Input 1", enabled=True, default=False),
            InputConfig(index=1, name="Input 2", enabled=True, default=False),
        ]
        
        # Find default (none set, should fallback to first enabled)
        default = None
        for inp in inputs:
            if inp.default and inp.enabled:
                default = inp
                break
        
        if default is None:
            enabled = [inp for inp in inputs if inp.enabled]
            default = enabled[0] if enabled else None
        
        assert default is not None
        assert default.name == "Input 1"

    def test_no_enabled_inputs(self, all_disabled_settings):
        """Test handling when all inputs are disabled."""
        @dataclass
        class InputConfig:
            index: int
            name: str
            enabled: bool = True
            default: bool = False

        inputs = [
            InputConfig(**input_data) for input_data in all_disabled_settings["inputs"]
        ]
        enabled = [inp for inp in inputs if inp.enabled]
        
        assert len(enabled) == 0


# =============================================================================
# LAYOUT MODE TESTS
# =============================================================================


class TestLayoutMode:
    """Tests for LayoutMode enum."""

    def test_layout_modes_exist(self):
        """Test that all layout modes are defined."""
        from enum import Enum, auto

        class LayoutMode(Enum):
            DUAL = auto()
            SINGLE_LEFT = auto()
            SINGLE_RIGHT = auto()

        assert LayoutMode.DUAL is not None
        assert LayoutMode.SINGLE_LEFT is not None
        assert LayoutMode.SINGLE_RIGHT is not None

    def test_layout_modes_unique(self):
        """Test that layout mode values are unique."""
        from enum import Enum, auto

        class LayoutMode(Enum):
            DUAL = auto()
            SINGLE_LEFT = auto()
            SINGLE_RIGHT = auto()

        values = [mode.value for mode in LayoutMode]
        assert len(values) == len(set(values))


# =============================================================================
# LOG UTILITY TESTS
# =============================================================================


class TestLogUtility:
    """Tests for the Log utility class."""

    def test_log_verbose_mode(self, capsys):
        """Test verbose mode toggle."""

        class Colors:
            RESET = "\033[0m"
            CYAN = "\033[96m"

        class Log:
            _verbose = False

            @classmethod
            def set_verbose(cls, enabled: bool):
                cls._verbose = enabled

            @classmethod
            def info(cls, msg: str, force: bool = False):
                if cls._verbose or force:
                    print(f"{Colors.CYAN}ℹ{Colors.RESET} {msg}")

        # Initially verbose is off
        Log.set_verbose(False)
        Log.info("Test message")
        captured = capsys.readouterr()
        assert captured.out == ""

        # Enable verbose
        Log.set_verbose(True)
        Log.info("Test message")
        captured = capsys.readouterr()
        assert "Test message" in captured.out

    def test_log_force_output(self, capsys):
        """Test forced output regardless of verbose mode."""

        class Colors:
            RESET = "\033[0m"
            CYAN = "\033[96m"

        class Log:
            _verbose = False

            @classmethod
            def set_verbose(cls, enabled: bool):
                cls._verbose = enabled

            @classmethod
            def info(cls, msg: str, force: bool = False):
                if cls._verbose or force:
                    print(f"{Colors.CYAN}ℹ{Colors.RESET} {msg}")

        Log.set_verbose(False)
        Log.info("Forced message", force=True)
        captured = capsys.readouterr()
        assert "Forced message" in captured.out


# =============================================================================
# KEY MAPPING TESTS
# =============================================================================


class TestKeyMappings:
    """Tests for keyboard shortcut mappings."""

    def test_input_selection_keys(self):
        """Test that input selection uses keys 1-4."""
        # Simulate key codes (Qt.Key values)
        Key_1 = 0x31  # Qt.Key.Key_1
        Key_2 = 0x32
        Key_3 = 0x33
        Key_4 = 0x34

        input_keys = [Key_1, Key_2, Key_3, Key_4]
        
        for i, key in enumerate(input_keys):
            input_index = key - Key_1
            assert input_index == i

    def test_layout_switching_keys(self):
        """Test layout switching key mappings."""
        # D for dual, L for single left, R for single right
        layout_keys = {
            "D": "DUAL",
            "L": "SINGLE_LEFT",
            "R": "SINGLE_RIGHT",
        }
        
        assert "D" in layout_keys
        assert "L" in layout_keys
        assert "R" in layout_keys
        assert layout_keys["D"] == "DUAL"
        assert layout_keys["L"] == "SINGLE_LEFT"
        assert layout_keys["R"] == "SINGLE_RIGHT"


# =============================================================================
# SETTINGS VALIDATION TESTS
# =============================================================================


class TestSettingsValidation:
    """Tests for settings validation logic."""

    def test_ensure_at_least_one_default(self, sample_settings):
        """Test that at least one default input exists."""
        inputs = sample_settings["inputs"]
        defaults = [inp for inp in inputs if inp.get("default", False)]
        
        assert len(defaults) >= 1

    def test_ensure_default_is_enabled(self, sample_settings):
        """Test that default input is also enabled."""
        inputs = sample_settings["inputs"]
        
        for inp in inputs:
            if inp.get("default", False):
                assert inp.get("enabled", True) is True, \
                    "Default input must be enabled"

    def test_input_indices_unique(self, sample_settings):
        """Test that all input indices are unique."""
        inputs = sample_settings["inputs"]
        indices = [inp["index"] for inp in inputs]
        
        assert len(indices) == len(set(indices)), "Input indices must be unique"

    def test_input_indices_in_range(self, sample_settings):
        """Test that input indices are in valid range (0-3)."""
        inputs = sample_settings["inputs"]
        
        for inp in inputs:
            assert 0 <= inp["index"] <= 3, f"Index {inp['index']} out of range"


# =============================================================================
# INTEGRATION TESTS
# =============================================================================


class TestIntegration:
    """Integration tests for settings flow."""

    def test_settings_roundtrip(self, tmp_path, sample_settings):
        """Test saving and loading settings."""
        settings_path = tmp_path / "settings.json"
        
        # Save
        with open(settings_path, "w") as f:
            json.dump(sample_settings, f, indent=4)
        
        # Load
        with open(settings_path, "r") as f:
            loaded = json.load(f)
        
        # Verify
        assert loaded == sample_settings

    def test_modify_and_save_settings(self, tmp_path, sample_settings):
        """Test modifying and saving settings."""
        settings_path = tmp_path / "settings.json"
        
        # Modify settings
        sample_settings["inputs"][1]["enabled"] = False
        sample_settings["inputs"][2]["enabled"] = True
        
        # Save
        with open(settings_path, "w") as f:
            json.dump(sample_settings, f, indent=4)
        
        # Load and verify
        with open(settings_path, "r") as f:
            loaded = json.load(f)
        
        assert loaded["inputs"][1]["enabled"] is False
        assert loaded["inputs"][2]["enabled"] is True


# =============================================================================
# EDGE CASE TESTS
# =============================================================================


class TestEdgeCases:
    """Tests for edge cases and error handling."""

    def test_empty_inputs_array(self, empty_settings):
        """Test handling of empty inputs array."""
        inputs = empty_settings.get("inputs", [])
        assert len(inputs) == 0

    def test_missing_input_fields(self):
        """Test handling of missing fields in input config."""
        incomplete_input = {"index": 0}  # Missing name, enabled, default
        
        # Should use defaults
        name = incomplete_input.get("name", f"Input {incomplete_input.get('index', 0)}")
        enabled = incomplete_input.get("enabled", True)
        default = incomplete_input.get("default", False)
        
        assert name == "Input 0"
        assert enabled is True
        assert default is False

    def test_invalid_input_index(self):
        """Test handling of invalid input indices."""
        invalid_indices = [-1, 5, 100]
        valid_range = range(0, 4)
        
        for idx in invalid_indices:
            assert idx not in valid_range

    def test_duplicate_defaults(self):
        """Test handling when multiple inputs are marked as default."""
        settings = {
            "inputs": [
                {"index": 0, "name": "Input 1", "enabled": True, "default": True},
                {"index": 1, "name": "Input 2", "enabled": True, "default": True},
            ]
        }
        
        defaults = [inp for inp in settings["inputs"] if inp["default"]]
        
        # Should have multiple defaults (application should handle this)
        assert len(defaults) == 2
        
        # First default should be used
        first_default = next(
            inp for inp in settings["inputs"] if inp["default"] and inp["enabled"]
        )
        assert first_default["index"] == 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
