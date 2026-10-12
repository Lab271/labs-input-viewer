# 0045. Release by hand; derive the version bump from Conventional Commits, with an explicit override

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-04-17 (82baf03, #38, manual release); 2026-08-25 (#290, bump input) |
| **Area** | Delivery |
| **Evidence** | `.github/workflows/release.yml:3-26, 53-91`; `.github/workflows/ci.yml:3-7`; `.claude/CLAUDE.md`, `.claude/DEVELOPMENT.md`; #250 |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

Releasing on every merge produced **13 patch releases on 2026-01-16 alone**
(v2.1.0 to v2.1.13). Auto-merge with a PAT was tried and reverted. Commit
analysis reads only subjects, so a `BREAKING CHANGE:` footer is never seen,
and `force_release` could rescue a `none` but never raise the level: shipping
3.0.0 would have meant writing a false `!:` (#290).

## Decision

- CI runs on PRs to `main` and pushes to `main`, nothing else. A stacked PR
  gets no checks until retargeted.
- Releasing is `gh workflow run release.yml` only; nothing triggers on a tag.
- Bump: `!:` or `BREAKING CHANGE` -> major, `feat` -> minor,
  `fix|perf|refactor|build|ci` -> patch, anything else -> none. A `bump`
  input (patch/minor/major) skips analysis.
- Branches `type/short-description`; no commitlint -- the release regex is
  the only consumer.

## Consequences

- **Dependabot's `chore(deps*)` commits never bump.** Electron -- Chromium's
  security fixes -- is a devDependency, so an Electron-only change computes
  `none`; v3.4.1 must have been dispatched with an explicit `patch`.
- `force_release` with no bump builds and then never publishes (the publish
  job needs the bump job) -- likely a latent bug.

## Porting to egui

**Keep.** Prototype (`release-egui.yml`): manual dispatch; auto bump from
commits that touch `input_viewer_egui/` since the last `egui-v*` tag; an
explicit `bump` input; a no-bump run fails loudly rather than building and
discarding. With fewer runtime security updates in a native binary the
Dependabot gap matters less, but `chore(deps)` on a Cargo dependency with a
security fix still needs a manual patch release.
