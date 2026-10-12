# 0051. Ask for camera access before opening any capture

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2025-12-30 (3b5bd3d) |
| **Area** | Platform |
| **Evidence** | `src/main/index.js:328-351`; `build/entitlements.mac.plist`; `renderer.js:865` |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

On macOS, capture needs a TCC grant; on Windows, only the Settings privacy
toggles for desktop apps.

## Decision

On macOS, await `askForMediaAccess('camera')` before creating the window, so
the first capture does not race the permission prompt. Microphone access is
not asked up front, although audio is captured. A denied grant is ignored:
the window opens anyway. Rationale beyond the race is not recorded.

## Consequences

The microphone prompt appears later, at the first audio capture.

## Porting to egui

**Keep**, and ask for both. Prototype: `nokhwa_initialize` requests camera
access before enumerating, and the bundled `Info.plist` carries
`NSCameraUsageDescription` -- without it macOS kills a bundled app on its
first camera access. Add `NSMicrophoneUsageDescription` and the microphone
request with audio ([0009](0009-pair-audio-with-its-video-card.md)). Note
that macOS attributes the grant to the launching app: run from a terminal
that can show the prompt, or from the installed bundle.
