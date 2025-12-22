"""
Colored logging utility for terminal output.
"""


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
    """Simple colored logging utility with verbose mode support."""

    _verbose = False

    @classmethod
    def set_verbose(cls, enabled: bool):
        """Enable or disable verbose logging."""
        cls._verbose = enabled

    @classmethod
    def _print(cls, color: str, prefix: str, msg: str, force: bool = False):
        """Print a colored message if verbose mode is enabled or force is True."""
        if cls._verbose or force:
            print(f"{color}{prefix}{Colors.RESET} {msg}")

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
