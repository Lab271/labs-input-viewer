"""
Entry point for HDMI Viewer application.

Usage:
    python -m hdmi_viewer                  # Production mode (real cameras)
    python -m hdmi_viewer --verbose        # With verbose logging

Keyboard shortcuts:
    F11 / F  - Toggle fullscreen
    Escape   - Exit fullscreen (or quit if windowed)
    Q        - Quit

    Layout switching:
    D        - Dual view (both feeds side by side)
    L        - Single view: left feed centered
    R        - Single view: right feed centered

    Input selection:
    1-4      - Select input directly
    T        - Show input thumbnails panel

    Features:
    Space    - Freeze frame
    A        - Toggle auto-switch
"""

import argparse
import sys
import traceback

from PyQt6.QtWidgets import QApplication

from hdmi_viewer.log import Log
from hdmi_viewer.viewer import DualVideoViewer


def exception_handler(exc_type, exc_value, exc_tb):
    """Global exception handler to log crashes."""
    error_msg = "".join(traceback.format_exception(exc_type, exc_value, exc_tb))
    Log.error(f"CRASH: {exc_type.__name__}: {exc_value}", force=True)
    Log.error(f"Traceback:\n{error_msg}", force=True)
    # Also print to stderr in case logging fails
    print(f"CRASH: {error_msg}", file=sys.stderr)
    sys.__excepthook__(exc_type, exc_value, exc_tb)


def parse_args():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="Dual Elgato Capture Card Viewer for Ultrawide Displays"
    )

    # Test mode group
    test_group = parser.add_mutually_exclusive_group()
    test_group.add_argument(
        "--mock",
        "-m",
        action="store_true",
        help="Run in test mode with mock video sources (animated pattern)",
    )
    test_group.add_argument(
        "--switch-signals",
        "-s",
        action="store_true",
        help="Test mode with signal cycling (10s signal, 10s no-signal)",
    )
    test_group.add_argument(
        "--no-signal",
        "-n",
        action="store_true",
        help="Test mode with always no-signal state",
    )

    # Verbose mode
    parser.add_argument(
        "--verbose",
        "-v",
        action="store_true",
        help="Enable verbose output with colored logging",
    )

    return parser.parse_args()


def main():
    """Main entry point."""
    args = parse_args()

    # Set verbose mode
    Log.set_verbose(args.verbose)

    # Enable file logging for bundled apps (helps debug crashes)
    if hasattr(sys, "_MEIPASS"):
        Log.enable_file_logging()
        sys.excepthook = exception_handler

    # Determine test mode
    test_mode = args.mock or args.switch_signals or args.no_signal
    switch_signals = args.switch_signals
    always_no_signal = args.no_signal

    if test_mode:
        if always_no_signal:
            Log.header("NO-SIGNAL MODE - Always showing no-signal overlay", force=True)
        elif switch_signals:
            Log.header(
                "SWITCH-SIGNALS MODE - Cycling signal/no-signal every 10s", force=True
            )
        else:
            Log.header("MOCK MODE - Using mock video sources", force=True)
        Log.info("Layout: D=dual, L=single left, R=single right", force=True)
        Log.info("Input: 1-4 to select, T for thumbnails", force=True)

    app = QApplication(sys.argv)
    app.setStyle("Fusion")

    viewer = DualVideoViewer(
        test_mode=test_mode,
        switch_signals=switch_signals,
        always_no_signal=always_no_signal,
    )
    viewer.show()

    sys.exit(app.exec())


if __name__ == "__main__":
    main()
