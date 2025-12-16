# -*- mode: python ; coding: utf-8 -*-
"""
PyInstaller spec file for Space Presenter

This spec file creates a properly bundled application with all resources.
"""

import os
import sys

block_cipher = None

# Get the directory containing the spec file
spec_dir = os.path.dirname(os.path.abspath(SPEC))

# Data files to include
datas = [
    ('Logo-3-OnDark.png', '.'),
    ('no_signal_icon.png', '.'),
    ('settings.json', '.'),
]

# Add no_signal_frames if it exists
if os.path.exists(os.path.join(spec_dir, 'no_signal_frames')):
    datas.append(('no_signal_frames', 'no_signal_frames'))

a = Analysis(
    ['HDMI-viewer.py'],
    pathex=[],
    binaries=[],
    datas=datas,
    hiddenimports=[
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
    ],
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
    name='Space Presenter',
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
        name='Space Presenter.app',
        icon='assets/icon.icns',
        bundle_identifier='com.lab271.space-presenter',
        info_plist={
            'CFBundleName': 'Space Presenter',
            'CFBundleDisplayName': 'Space Presenter',
            'CFBundleVersion': open('VERSION').read().strip(),
            'CFBundleShortVersionString': open('VERSION').read().strip(),
            'NSHighResolutionCapable': True,
            'NSCameraUsageDescription': 'Space Presenter needs camera access to display capture card feeds.',
        },
    )
