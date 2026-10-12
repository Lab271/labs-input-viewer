# 0046. VERSION is the source of truth; bump through a PR, tag the merged commit

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-03 (#84, protected main); 2026-08-11 (#197, lock file); 2026-01-23 (#19, build with the new version) |
| **Area** | Delivery |
| **Evidence** | `release.yml:44-51, 170-248, 253-347`; `VERSION`; `CHANGELOG.md` |
| **egui verdict** | Adapt |
| **egui status** | Partial |

## Context

- After branch protection, v2.6.2's release failed with `GH006 Protected
  branch update failed`; nothing shipped between 2026-06-08 and 2026-08-03,
  including a security fix (#69).
- The bump staged VERSION and `package.json` but not the lock file, which sat
  at 2.9.0 while the others reached 2.13.0 for four releases; `npm ci`
  tolerates the mismatch, so nothing failed.
- Why there is a separate VERSION file is not recorded; it predates
  Electron (the Python v1).

## Decision

- Builds run in parallel on the pre-bump commit with the new version written
  in on the runner.
- Then: push `chore/bump-vX` with VERSION, `package.json`, `package-lock.json`;
  open a PR and squash-merge it at once; refetch `main`; **refuse to tag**
  unless `main:VERSION` is the new version; push an annotated `vX.Y.Z`.
- Publish only if the bump succeeded -- otherwise the release action would
  create the tag itself on an un-bumped `main`.
- A hand-written Keep-a-Changelog, updated after the fact; the release page
  uses GitHub's generated notes.

## Consequences

The bump PR merges only because `main` requires no reviews and no status
checks; adding either would break releasing. The changelog lags (it stopped
at 2.13.0 and missed 2.14-2.19 once).

## Porting to egui

**Adapt.** The prototype makes the **tag** the source of truth: `Cargo.toml`
stays `0.0.0` in git and the release workflow writes the version into
`Cargo.toml` and `Cargo.lock` at build time (both, for the lock-file reason
above). There is no bump commit, so no protected-branch dance. Tags are
`egui-v*` so they never become the Electron workflow's baseline -- which
needed `git describe --match 'v[0-9]*'` in `release.yml`.

Decide whether the egui app should rejoin VERSION when it replaces Electron.
A rewrite traditionally starts a new major and a new CHANGELOG section, as
Python -> Electron did.
