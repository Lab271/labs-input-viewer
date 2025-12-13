#!/usr/bin/env python3
"""Create icon files for all platforms from base_icon.png"""

import os
import subprocess
from PIL import Image


def main():
    # Get the project root
    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(script_dir)
    assets_dir = os.path.join(project_root, "assets")

    # Load the base icon
    base_icon_path = os.path.join(assets_dir, "base_icon.png")
    img = Image.open(base_icon_path)
    print(f"Loaded base_icon.png: {img.size}, mode: {img.mode}")

    # Convert to RGBA if needed (for transparency support)
    if img.mode != "RGBA":
        img = img.convert("RGBA")

    # Create icon.png (512x512 for general use)
    icon_512 = img.resize((512, 512), Image.Resampling.LANCZOS)
    icon_512.save(os.path.join(assets_dir, "icon.png"), "PNG")
    print("✓ Created icon.png (512x512)")

    # Create Windows ICO file (multiple sizes embedded)
    ico_sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    ico_images = [img.resize(size, Image.Resampling.LANCZOS) for size in ico_sizes]
    ico_images[0].save(
        os.path.join(assets_dir, "icon.ico"), format="ICO", sizes=ico_sizes
    )
    print("✓ Created icon.ico (Windows, multi-size)")

    # Create macOS ICNS using iconutil
    iconset_path = os.path.join(assets_dir, "icon.iconset")
    os.makedirs(iconset_path, exist_ok=True)

    # macOS iconset requires specific filenames and sizes
    icns_sizes = [
        ("icon_16x16.png", 16),
        ("icon_16x16@2x.png", 32),
        ("icon_32x32.png", 32),
        ("icon_32x32@2x.png", 64),
        ("icon_128x128.png", 128),
        ("icon_128x128@2x.png", 256),
        ("icon_256x256.png", 256),
        ("icon_256x256@2x.png", 512),
        ("icon_512x512.png", 512),
        ("icon_512x512@2x.png", 1024),
    ]

    for filename, size in icns_sizes:
        resized = img.resize((size, size), Image.Resampling.LANCZOS)
        resized.save(os.path.join(iconset_path, filename), "PNG")

    print(f"✓ Created iconset folder with {len(icns_sizes)} sizes")

    # Run iconutil to create .icns (macOS only)
    icns_path = os.path.join(assets_dir, "icon.icns")
    try:
        subprocess.run(
            ["iconutil", "-c", "icns", iconset_path, "-o", icns_path],
            check=True,
            capture_output=True,
        )
        print("✓ Created icon.icns (macOS)")

        # Clean up iconset folder
        import shutil

        shutil.rmtree(iconset_path)
        print("✓ Cleaned up iconset folder")
    except FileNotFoundError:
        print("⚠ iconutil not found (not on macOS). Iconset folder kept for manual conversion.")
    except subprocess.CalledProcessError as e:
        print(f"⚠ iconutil failed: {e.stderr.decode()}")

    print("\nDone! Icon files created in assets/")


if __name__ == "__main__":
    main()
