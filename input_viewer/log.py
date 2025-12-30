"""
Colored logging utility for terminal output with optional file logging.
"""

import os
import sys
from datetime import datetime


def _get_log_file_path():
    """Get the path for the log file."""
    if sys.platform == "win32":
        log_dir = os.path.join(os.environ.get("APPDATA", ""), "Space Presenter")
    elif sys.platform == "darwin":
        log_dir = os.path.expanduser("~/Library/Logs/Space Presenter")
    else:
        log_dir = os.path.expanduser("~/.local/share/input-viewer")
    os.makedirs(log_dir, exist_ok=True)
    return os.path.join(log_dir, "app.log")


class Colors:
    """ANSI color codes for terminal output."""

    RESET = "\033[0m"
    BOLD = "\033[1m"
    DIM = "\033[2m"
    RED = "\033[91m"
    GREEN = "\033[92m"
    YELLOW = "\033[93m"
    BLUE = "\033[94m"
    MAGENTA = "\033[95m"
    CYAN = "\033[96m"
    WHITE = "\033[97m"


class Log:
    """Simple colored logging utility with verbose mode support and file logging."""

    _verbose = False
    _file_logging = False
    _log_file = None

    @classmethod
    def enable_file_logging(cls):
        """Enable logging to file (useful for debugging crashes)."""
        cls._file_logging = True
        log_path = _get_log_file_path()
        try:
            cls._log_file = open(log_path, "a", encoding="utf-8")
            cls._write_to_file(f"\n{'='*60}")
            cls._write_to_file(f"Session started: {datetime.now().isoformat()}")
            cls._write_to_file(f"{'='*60}")
            print(f"Logging to: {log_path}")
        except Exception as e:
            print(f"Failed to open log file: {e}")
            cls._file_logging = False

    @classmethod
    def _write_to_file(cls, msg: str):
        """Write message to log file if enabled."""
        if cls._file_logging and cls._log_file:
            try:
                timestamp = datetime.now().strftime("%H:%M:%S.%f")[:-3]
                cls._log_file.write(f"[{timestamp}] {msg}\n")
                cls._log_file.flush()
            except Exception:
                pass

    @classmethod
    def set_verbose(cls, enabled: bool):
        """Enable or disable verbose logging."""
        cls._verbose = enabled

    @classmethod
    def _print(cls, color: str, prefix: str, msg: str, force: bool = False):
        """Print a colored message if verbose mode is enabled or force is True."""
        if cls._verbose or force:
            print(f"{color}{prefix}{Colors.RESET} {msg}")
        # Always write errors and warnings to file, others only if verbose
        if cls._file_logging and (force or cls._verbose or prefix in ("✗", "⚠")):
            cls._write_to_file(f"{prefix} {msg}")

    @classmethod
    def info(cls, msg: str, force: bool = False):
        """Print info message (cyan)."""
        cls._print(Colors.CYAN, "ℹ", msg, force)

    @classmethod
    def success(cls, msg: str, force: bool = False):
        """Print success message (green)."""
        cls._print(Colors.GREEN, "✓", msg, force)

    @classmethod
    def warning(cls, msg: str, force: bool = False):
        """Print warning message (yellow)."""
        cls._print(Colors.YELLOW, "⚠", msg, force)

    @classmethod
    def error(cls, msg: str, force: bool = False):
        """Print error message (red)."""
        cls._print(Colors.RED, "✗", msg, force)

    @classmethod
    def debug(cls, msg: str):
        """Print debug message (dim, only in verbose mode)."""
        if cls._verbose:
            print(f"{Colors.DIM}  {msg}{Colors.RESET}")

    @classmethod
    def header(cls, msg: str, force: bool = False):
        """Print a header message (bold magenta)."""
        if cls._verbose or force:
            print(f"{Colors.BOLD}{Colors.MAGENTA}{'=' * 60}{Colors.RESET}")
            print(f"{Colors.BOLD}{Colors.MAGENTA}{msg}{Colors.RESET}")
            print(f"{Colors.BOLD}{Colors.MAGENTA}{'=' * 60}{Colors.RESET}")
