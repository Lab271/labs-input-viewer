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
    
    if detector.reference_features is None:
        print("ERROR: Reference image not loaded!")
        ref_path = get_resource_path("elgato_no_source.png")
        print(f"  Expected at: {ref_path}")
        print(f"  File exists: {os.path.exists(ref_path)}")
        return
    
    print(f"Reference image loaded successfully")
    print(f"  Feature vector size: {len(detector.reference_features)}")
    print(f"  Similarity threshold: {detector.SIMILARITY_THRESHOLD}")
    
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
        
        # Run detection
        is_no_signal = detector.is_no_signal(frame)
        
        # Calculate similarity for display
        frame_features = detector._extract_features(frame)
        similarity = detector._cosine_similarity(frame_features, detector.reference_features)
        
        if is_no_signal:
            detected_count += 1
        
        # Draw info on frame
        color = (0, 0, 255) if is_no_signal else (0, 255, 0)
        status = "NO SIGNAL DETECTED" if is_no_signal else "Signal OK"
        
        # Create info overlay
        cv2.rectangle(frame, (10, 10), (400, 120), (0, 0, 0), -1)
        cv2.putText(frame, status, (20, 45), 
                    cv2.FONT_HERSHEY_SIMPLEX, 1.0, color, 2)
        cv2.putText(frame, f"Similarity: {similarity:.4f}", (20, 75),
                    cv2.FONT_HERSHEY_SIMPLEX, 0.7, (255, 255, 255), 1)
        cv2.putText(frame, f"Threshold:  {detector.SIMILARITY_THRESHOLD:.4f}", (20, 100),
                    cv2.FONT_HERSHEY_SIMPLEX, 0.7, (255, 255, 255), 1)
        
        # Border color based on detection
        cv2.rectangle(frame, (0, 0), (w-1, h-1), color, 3)
        
        # Show frame
        cv2.imshow("Detection Test", frame)
        
        # Print to console every 30 frames
        if frame_count % 30 == 0:
            pct = (detected_count / frame_count) * 100
            print(f"Frame {frame_count}: similarity={similarity:.4f}, "
                  f"detected={is_no_signal}, "
                  f"detection_rate={pct:.1f}%")
        
        # Handle keyboard input
        key = cv2.waitKey(1) & 0xFF
        
        if key == ord('q'):
            break
        elif key == ord('s'):
            filename = f"debug_frame_{frame_count}.png"
            cv2.imwrite(filename, frame)
            print(f"Saved: {filename}")
        elif key == ord('t'):
            print(f"\n--- Threshold Info ---")
            print(f"Current similarity: {similarity:.4f}")
            print(f"Threshold: {detector.SIMILARITY_THRESHOLD}")
            print(f"Detected: {is_no_signal}")
            print(f"Gap: {similarity - detector.SIMILARITY_THRESHOLD:+.4f}")
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
    
    # Test with original
    is_detected = detector.is_no_signal(ref_img)
    features = detector._extract_features(ref_img)
    similarity = detector._cosine_similarity(features, detector.reference_features)
    
    print(f"Reference image: {ref_path}")
    print(f"  Size: {ref_img.shape[1]}x{ref_img.shape[0]}")
    print(f"  Self-similarity: {similarity:.4f}")
    print(f"  Detected: {is_detected}")
    
    # Test with scaled version
    scaled = cv2.resize(ref_img, (1920, 1080))
    is_detected_scaled = detector.is_no_signal(scaled)
    features_scaled = detector._extract_features(scaled)
    similarity_scaled = detector._cosine_similarity(features_scaled, detector.reference_features)
    
    print(f"\nScaled to 1920x1080:")
    print(f"  Similarity: {similarity_scaled:.4f}")
    print(f"  Detected: {is_detected_scaled}")
    
    # Test with slight modifications
    noisy = ref_img.copy()
    noise = np.random.randint(-10, 10, ref_img.shape, dtype=np.int16)
    noisy = np.clip(noisy.astype(np.int16) + noise, 0, 255).astype(np.uint8)
    
    is_detected_noisy = detector.is_no_signal(noisy)
    features_noisy = detector._extract_features(noisy)
    similarity_noisy = detector._cosine_similarity(features_noisy, detector.reference_features)
    
    print(f"\nWith slight noise:")
    print(f"  Similarity: {similarity_noisy:.4f}")
    print(f"  Detected: {is_detected_noisy}")


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
