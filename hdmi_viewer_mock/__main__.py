"""
Mock server entry point.

Run with: python -m hdmi_viewer_mock
"""

import argparse
import sys


def parse_args():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="HDMI Viewer Mock - Virtual camera server for testing",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s                  Start with GUI control panel
  %(prog)s --headless       Start in headless mode (no GUI)
  %(prog)s --pattern bars   Start with color bars pattern
  %(prog)s --no-signal      Start showing no-signal screen
        """,
    )

    parser.add_argument(
        "--headless",
        action="store_true",
        help="Run without GUI control panel",
    )

    parser.add_argument(
        "--pattern",
        choices=["bars", "gradient", "bouncing", "static"],
        default="bars",
        help="Initial test pattern (default: bars)",
    )

    parser.add_argument(
        "--no-signal",
        action="store_true",
        help="Start in no-signal mode",
    )

    parser.add_argument(
        "--width",
        type=int,
        default=1920,
        help="Output width in pixels (default: 1920)",
    )

    parser.add_argument(
        "--height",
        type=int,
        default=1080,
        help="Output height in pixels (default: 1080)",
    )

    parser.add_argument(
        "--fps",
        type=int,
        default=30,
        help="Output frame rate (default: 30)",
    )

    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Enable verbose output",
    )

    return parser.parse_args()


def main():
    """Main entry point."""
    args = parse_args()

    # Check for pyvirtualcam
    import importlib.util
    if importlib.util.find_spec("pyvirtualcam") is None:
        print("Error: pyvirtualcam is required for the mock server.")
        print("Install it with: pip install pyvirtualcam")
        print("")
        print("Platform-specific requirements:")
        print("  macOS: brew install obs (for OBS Virtual Camera)")
        print("  Linux: sudo apt install v4l2loopback-dkms")
        print("  Windows: Install OBS Studio")
        sys.exit(1)

    from .server import run_mock_server

    run_mock_server(
        headless=args.headless,
        pattern=args.pattern,
        no_signal=args.no_signal,
        width=args.width,
        height=args.height,
        fps=args.fps,
        verbose=args.verbose,
    )


if __name__ == "__main__":
    main()
