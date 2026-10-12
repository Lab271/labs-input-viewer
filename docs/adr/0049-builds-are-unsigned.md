# 0049. Builds are unsigned

| | |
|---|---|
| **Status** | Accepted (by default) |
| **Date** | not recorded; #56 open and on hold |
| **Area** | Delivery |
| **Evidence** | `release.yml:196-199` (`CSC_IDENTITY_AUTO_DISCOVERY: false`); `build/entitlements.mac.plist`; `SECURITY.md`; #56, #86 |
| **egui verdict** | Decide |
| **egui status** | Same as Electron (ad-hoc on macOS) |

## Context

No Apple Developer ID or Authenticode certificate is configured. The rationale
is not recorded; presumably there is no certificate. `hardenedRuntime` and
entitlements are configured anyway, to little effect while unsigned.

## Decision

Ship unsigned. electron-updater verifies only the SHA-512 that sits in the
same feed, so trust in an update is write access to the GitHub repo.

## Consequences

- Gatekeeper and SmartScreen warnings on install.
- macOS auto-update (Squirrel.Mac validates the signature) has **never been
  confirmed end to end**: #86's checkbox "confirm an installed macOS client
  actually updates" is unchecked.
- The wall is Windows, so the practical cost lands on macOS users.

## Porting to egui

**Decide**, consciously this time. The prototype signs the macOS app ad hoc
(`codesign --sign -`) because Apple silicon will not run an unsigned arm64
binary, and leaves the Windows installer unsigned. Velopack can sign and
notarise if certificates exist. The JIT and library-validation entitlements
are V8's and are not needed; the camera usage string is
([0051](0051-request-capture-permission-up-front.md)).
