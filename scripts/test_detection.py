#!/usr/bin/env python3
"""
Isolated test for Elgato no-signal detection.

Usage:
    python scripts/test_detection.py [camera_index]
    
    camera_index: Optional, defaults to 0. Use -1 to list available cameras.

Controls:
    q - Quit
    s - Save current frame for debugging
    t - Show similarity threshold info
"""

import sys
import os

# Add parent directory to path for imports
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import cv2
import numpy as np
from hdmi_viewer.detection import NoSignalDetector
from hdmi_viewer.utils import get_resource_path


def list_cameras(max_cameras=10):
    """List available camera indices."""
    print("\nScanning for available cameras...")
    available = []
    for i in range(max_cameras):
        cap = cv2.VideoCapture(i)
        if cap.isOpened():
            ret, _ = cap.read()
            if ret:
                available.append(i)
                # Get camera info
                w = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
                h = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))
                fps = cap.get(cv2.CAP_PROP_FPS)
                print(f"  Camera {i}: {w}x{h} @ {fps:.1f}fps")
            cap.release()
    
    if not available:
        print("  No cameras found!")
    return available


def test_detection(camera_index=0):
    """Run detection test on camera feed."""
    
    # Initialize detector
    print("\n=== Elgato No-Signal Detection Test ===\n")
    
    detector = NoSignalDetector()
    
    if detector.logo_template is None:
        print("ERROR: Logo template not loaded!")
        ref_path = get_resource_path("elgato_no_source.png")
        print(f"  Expected at: {ref_path}")
        print(f"  File exists: {os.path.exists(ref_path)}")
        return
    
    print(f"Logo template loaded successfully")
    print(f"  Template size: {detector.logo_template.shape[1]}x{detector.logo_template.shape[0]}")
    print(f"  Match threshold: {detector.MATCH_THRESHOLD}")
    print(f"  Background grey threshold: {detector.BACKGROUND_GREY_RATIO:.0%}")
    
    # Open camera
    print(f"\nOpening camera {camera_index}...")
    cap = cv2.VideoCapture(camera_index)
    
    if not cap.isOpened():
        print(f"ERROR: Could not open camera {camera_index}")
        print("\nAvailable cameras:")
        list_cameras()
        return
    
    # Get camera properties
    w = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
    h = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))
    fps = cap.get(cv2.CAP_PROP_FPS)
    print(f"  Resolution: {w}x{h}")
    print(f"  FPS: {fps}")
    
    print("\nControls:")
    print("  q - Quit")
    print("  s - Save current frame")
    print("  t - Show threshold info")
    print("\n" + "="*50)
    
    frame_count = 0
    detected_count = 0
    
    while True:
        ret, frame = cap.read()
        if not ret:
            print("Failed to read frame")
            break
        
        frame_count += 1
        
        # Get detailed detection info
        details = detector.get_detection_details(frame)
        is_no_signal = details["is_no_signal"]
        
        if is_no_signal:
            detected_count += 1
        
        # Draw info on frame
        color = (0, 0, 255) if is_no_signal else (0, 255, 0)
        status = "NO SIGNAL DETECTED" if is_no_signal else "Signal OK"
        
        # Create info overlay
        cv2.rectangle(frame, (10, 10), (450, 180), (0, 0, 0), -1)
        cv2.putText(frame, status, (20, 40), 
                    cv2.FONT_HERSHEY_SIMPLEX, 0.9, color, 2)
        
        # Logo match info
        logo_color = (0, 255, 0) if details["logo_found"] else (0, 0, 255)
        cv2.putText(frame, f"Logo match: {details['match_score']:.3f} (thresh: {details['match_threshold']:.2f})", 
                    (20, 70), cv2.FONT_HERSHEY_SIMPLEX, 0.6, logo_color, 1)
        
        # Background info
        bg_color = (0, 255, 0) if details["background_valid"] else (0, 0, 255)
        cv2.putText(frame, f"Background grey: {details['background_grey_ratio']:.1%} (thresh: {details['background_threshold']:.0%})", 
                    (20, 100), cv2.FONT_HERSHEY_SIMPLEX, 0.6, bg_color, 1)
        
        # Logo location info
        if details["logo_location"]:
            loc = details["logo_location"]
            size = details["logo_size"]
            cv2.putText(frame, f"Logo at: ({loc[0]}, {loc[1]}) size: {size[0]}x{size[1]}", 
                        (20, 130), cv2.FONT_HERSHEY_SIMPLEX, 0.5, (255, 255, 255), 1)
            
            # Draw rectangle around detected logo
            cv2.rectangle(frame, loc, (loc[0] + size[0], loc[1] + size[1]), (0, 255, 255), 2)
        
        cv2.putText(frame, f"Scale: {details['logo_scale']:.2f}" if details['logo_scale'] else "Scale: N/A", 
                    (20, 160), cv2.FONT_HERSHEY_SIMPLEX, 0.5, (255, 255, 255), 1)
        
        # Border color based on detection
        cv2.rectangle(frame, (0, 0), (w-1, h-1), color, 3)
        
        # Show frame
        cv2.imshow("Detection Test", frame)
        
        # Print to console every 30 frames
        if frame_count % 30 == 0:
            pct = (detected_count / frame_count) * 100
            print(f"Frame {frame_count}: logo={details['match_score']:.3f}, "
                  f"bg_grey={details['background_grey_ratio']:.1%}, "
                  f"detected={is_no_signal}, "
                  f"rate={pct:.1f}%")
        
        # Handle keyboard input
        key = cv2.waitKey(1) & 0xFF
        
        if key == ord('q'):
            break
        elif key == ord('s'):
            filename = f"debug_frame_{frame_count}.png"
            cv2.imwrite(filename, frame)
            print(f"Saved: {filename}")
            print(f"Details: {details}")
        elif key == ord('t'):
            print(f"\n--- Detection Details ---")
            for k, v in details.items():
                print(f"  {k}: {v}")
            print()
    
    cap.release()
    cv2.destroyAllWindows()
    
    print(f"\n=== Summary ===")
    print(f"Total frames: {frame_count}")
    print(f"Detected as no-signal: {detected_count}")
    print(f"Detection rate: {(detected_count/frame_count)*100:.1f}%")


def test_with_reference_image():
    """Test detection with the reference image itself (should always match)."""
    print("\n=== Testing with reference image ===\n")
    
    detector = NoSignalDetector()
    ref_path = get_resource_path("elgato_no_source.png")
    
    if not os.path.exists(ref_path):
        print(f"Reference image not found: {ref_path}")
        return
    
    ref_img = cv2.imread(ref_path)
    if ref_img is None:
        print(f"Failed to load reference image")
        return
    
    print(f"Reference image: {ref_path}")
    print(f"  Size: {ref_img.shape[1]}x{ref_img.shape[0]}")
    
    # Test with original
    details = detector.get_detection_details(ref_img)
    print(f"\nOriginal image detection:")
    for k, v in details.items():
        print(f"  {k}: {v}")
    
    # Test with scaled version
    scaled = cv2.resize(ref_img, (1920, 1080))
    details_scaled = detector.get_detection_details(scaled)
    print(f"\nScaled to 1920x1080:")
    for k, v in details_scaled.items():
        print(f"  {k}: {v}")
    
    # Test with different aspect ratio (simulate letterboxing)
    letterboxed = np.zeros((1080, 1920, 3), dtype=np.uint8) + 40  # Dark grey background
    # Place reference in center
    rh, rw = ref_img.shape[:2]
    scale = min(1200 / rw, 800 / rh)
    new_w, new_h = int(rw * scale), int(rh * scale)
    resized = cv2.resize(ref_img, (new_w, new_h))
    x_off = (1920 - new_w) // 2
    y_off = (1080 - new_h) // 2
    letterboxed[y_off:y_off+new_h, x_off:x_off+new_w] = resized
    
    details_letterbox = detector.get_detection_details(letterboxed)
    print(f"\nLetterboxed (logo centered in 1920x1080 with grey bars):")
    for k, v in details_letterbox.items():
        print(f"  {k}: {v}")


if __name__ == "__main__":
    if len(sys.argv) > 1:
        arg = sys.argv[1]
        if arg == "-1":
            list_cameras()
        elif arg == "--test-ref":
            test_with_reference_image()
        else:
            try:
                camera_idx = int(arg)
                test_detection(camera_idx)
            except ValueError:
                print(f"Invalid argument: {arg}")
                print("Usage: python test_detection.py [camera_index]")
                print("       python test_detection.py -1  (list cameras)")
                print("       python test_detection.py --test-ref  (test reference image)")
    else:
        # First test with reference image, then camera
        test_with_reference_image()
        print("\n" + "="*50)
        test_detection(0)
