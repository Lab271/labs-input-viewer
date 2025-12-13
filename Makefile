# HDMI Viewer Makefile
# ====================

.PHONY: help install install-dev run run-mock run-no-signal test lint format clean

# Default target
help:
	@echo "HDMI Viewer - Available commands:"
	@echo ""
	@echo "  make install       Install production dependencies"
	@echo "  make install-dev   Install development dependencies (includes test tools)"
	@echo "  make run           Run the application (production mode)"
	@echo "  make run-mock      Run with mock video sources"
	@echo "  make run-no-signal Run with no-signal test mode"
	@echo "  make run-verbose   Run with verbose logging"
	@echo "  make test          Run the test suite"
	@echo "  make test-verbose  Run tests with verbose output"
	@echo "  make lint          Run ruff linter"
	@echo "  make format        Format code with ruff"
	@echo "  make clean         Remove cache and build artifacts"
	@echo ""

# Python executable (use venv if available)
PYTHON := $(shell if [ -f venv/bin/python ]; then echo venv/bin/python; else echo python3; fi)
PIP := $(shell if [ -f venv/bin/pip ]; then echo venv/bin/pip; else echo pip3; fi)

# Installation
install:
	$(PIP) install PyQt6 opencv-python numpy Pillow

install-dev: install
	$(PIP) install pytest ruff

# Create virtual environment
venv:
	python3 -m venv venv
	@echo "Virtual environment created. Activate with: source venv/bin/activate"

# Running the application
run:
	$(PYTHON) HDMI-viewer.py

run-mock:
	$(PYTHON) HDMI-viewer.py --mock --verbose

run-no-signal:
	$(PYTHON) HDMI-viewer.py --no-signal --verbose

run-verbose:
	$(PYTHON) HDMI-viewer.py --verbose

run-switch:
	$(PYTHON) HDMI-viewer.py --switch-signals --verbose

# Testing
test:
	$(PYTHON) -m pytest tests/ -v

test-verbose:
	$(PYTHON) -m pytest tests/ -v -s

test-coverage:
	$(PYTHON) -m pytest tests/ -v --cov=. --cov-report=html

# Linting and formatting
lint:
	$(PYTHON) -m ruff check HDMI-viewer.py

lint-fix:
	$(PYTHON) -m ruff check HDMI-viewer.py --fix

format:
	$(PYTHON) -m ruff format HDMI-viewer.py

check: lint test
	@echo "All checks passed!"

# Cleaning
clean:
	rm -rf __pycache__
	rm -rf tests/__pycache__
	rm -rf .pytest_cache
	rm -rf .ruff_cache
	rm -rf htmlcov
	rm -rf .coverage
	find . -type f -name "*.pyc" -delete
	find . -type d -name "*.egg-info" -exec rm -rf {} + 2>/dev/null || true

# Generate no-signal frames (if needed)
generate-frames:
	@echo "Generating no-signal animation frames..."
	$(PYTHON) -c "from HDMI-viewer import CameraFeed; CameraFeed._generate_no_signal_frames()"
