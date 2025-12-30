"""
Input Viewer Mock - Virtual camera server for testing.

This package provides a virtual camera that can be used to test
the Input Viewer application without real capture hardware.
"""

__version__ = "1.0.0"


def main():
    """Entry point for the mock server."""
    from .server import run_mock_server
    run_mock_server()
