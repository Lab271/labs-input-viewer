# Input Viewer Makefile
# ====================

.PHONY: help install install-dev run run-mock test lint format clean build

# Default target
help:
	@echo "Input Viewer - Available commands:"
	@echo ""
	@echo "  make install       Install production dependencies"
	@echo "  make install-dev   Install development dependencies (includes test tools)"
	@echo "  make install-mock  Install mock server dependencies (includes pyvirtualcam)"
	@echo "  make run           Run the application (production mode)"
	@echo "  make run-mock      Run the mock video server"
	@echo "  make run-verbose   Run with verbose logging"
	@echo "  make test          Run the test suite"
	@echo "  make test-verbose  Run tests with verbose output"
	@echo "  make lint          Run ruff linter"
	@echo "  make format        Format code with ruff"
	@echo "  make clean         Remove cache and build artifacts"
	@echo "  make build         Build the application with PyInstaller"
	@echo ""

# Installation with uv
install:
	uv sync

install-dev:
	uv sync --extra dev

install-mock:
	uv sync --extra mock

install-all:
	uv sync --all-extras

# Running the application
run:
	uv run python input_viewer

run-verbose:
	uv run python -m input_viewer --verbose

# Running the mock server (separate process)
run-mock:
	uv run python -m input_viewer_mock

# Testing
test:
	uv run pytest tests/ -v

test-verbose:
	uv run pytest tests/ -v -s

test-coverage:
	uv run pytest tests/ -v --cov=input_viewer --cov-report=html

# Linting and formatting
lint:
	uv run ruff check input_viewer input_viewer_mock

lint-fix:
	uv run ruff check input_viewer input_viewer_mock --fix

format:
	uv run ruff format input_viewer input_viewer_mock

check: lint test
	@echo "All checks passed!"

# Building
build:
	uv run pyinstaller input-viewer.spec

# Cleaning
clean:
	rm -rf __pycache__
	rm -rf input_viewer/__pycache__
	rm -rf input_viewer/widgets/__pycache__
	rm -rf input_viewer_mock/__pycache__
	rm -rf tests/__pycache__
	rm -rf .pytest_cache
	rm -rf .ruff_cache
	rm -rf htmlcov
	rm -rf .coverage
	rm -rf build
	rm -rf dist
	find . -type f -name "*.pyc" -delete
	find . -type d -name "*.egg-info" -exec rm -rf {} + 2>/dev/null || true
	find . -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true