# -*- mode: python ; coding: utf-8 -*-
"""
PyInstaller spec file for Input Viewer

This spec file creates a properly bundled application with all resources.
"""

import os
import sys
from PyInstaller.utils.hooks import collect_submodules

block_cipher = None

# Get the directory containing the spec file
spec_dir = os.path.dirname(os.path.abspath(SPEC))

# Collect all input_viewer submodules
hidden_imports = collect_submodules('input_viewer')
hidden_imports.extend([
    'cv2',
    'numpy',
    'PIL',
    'PIL.Image',
    'PIL.ImageDraw',
    'PIL.ImageFont',
    'PyQt6',
    'PyQt6.QtCore',
    'PyQt6.QtGui',
    'PyQt6.QtWidgets',
])

# Data files to include
datas = [
    ('assets/logo.png', 'assets'),
    ('assets/no_signal_icon.png', 'assets'),
    ('assets/no_signal.mp4', 'assets'),
    ('assets/elgato_no_source.png', 'assets'),
    ('assets/zed.png', 'assets'),
    ('assets/icon.icns', 'assets'),
    ('assets/icon.ico', 'assets'),
    ('settings.json', '.'),
]

a = Analysis(
    ['input_viewer/__main__.py'],
    pathex=[spec_dir],
    binaries=[],
    datas=datas,
    hiddenimports=hidden_imports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.zipfiles,
    a.datas,
    [],
    name='Input Viewer',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    icon='assets/icon.icns' if sys.platform == 'darwin' else 'assets/icon.ico' if sys.platform == 'win32' else None,
)

# macOS app bundle
if sys.platform == 'darwin':
    app = BUNDLE(
        exe,
        name='Input Viewer.app',
        icon='assets/icon.icns',
        bundle_identifier='com.lab271.input-viewer',
        info_plist={
            'CFBundleName': 'Input Viewer',
            'CFBundleDisplayName': 'Input Viewer',
            'CFBundleVersion': open('VERSION').read().strip(),
            'CFBundleShortVersionString': open('VERSION').read().strip(),
            'NSHighResolutionCapable': True,
            'NSCameraUsageDescription': 'Input Viewer needs camera access to display capture card feeds.',
        },
    )
