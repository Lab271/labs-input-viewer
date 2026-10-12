# 0047. Gate every PR and release on lint, tests and build, and show the results inline

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-08-04 (#108, #109); 2026-07-22 (59c169b, vitest --no-save) |
| **Area** | Delivery |
| **Evidence** | `ci.yml:13-61`; `release.yml:123-165`; `scripts/test-summary.mjs`; `vitest.config.mjs`; `eslint.config.mjs`; `test/helpers/renderer-dom.js`; `renderer.js:5110` |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Context

Before #108 the release ran no tests, so a regression could be tagged while
CI on `main` was red. A jsdom worker that failed to boot once reported "26
passed" while whole suites were silently skipped.

## Decision

- The same job in both workflows: `npm ci`, tests with three reporters
  (console, `github-actions` for inline annotations on the diff, JUnit), a job
  summary rendered from JUnit (`if: always()`, never decides the run), lint,
  build. Unhandled errors fail the run.
- **Tests:** pure-logic suites run in node by default; renderer suites opt in
  to jsdom per file. `renderer.js` boots only without
  `__INPUT_VIEWER_NO_AUTOSTART__`, and the test DOM fixture derives its
  element ids from the renderer's source, so it cannot drift.
- **Lint for correctness, not style:** `js.configs.recommended` plus
  `no-eval`, `no-new-func`, `eqeqeq` (null ignored); three environments
  (main/preload CJS+Node, renderer ESM+browser, scripts/tests ESM+Node). No
  formatting rules, to avoid churning ~5,700 lines.
- **Toolchain pinned once:** Node 24 in both workflows, changed together.
- vitest is installed `--no-save`, not a devDependency. The stated reason
  (keep the lock in sync for `npm ci`) does not hold; the real one is not
  recorded. A bare `npm ci && npm test` therefore fails by design.

## Consequences

CI never packages, so packaging breaks surface only at release (#86).

## Porting to egui

**Keep.** Prototype: an `egui` job in `ci.yml` runs `cargo test --locked` and
a release build on macOS; `release-egui.yml` runs the tests before each
platform's build and builds the installers on every PR that touches the app
-- which closes the "packaging only breaks at release" gap. Missing:
`cargo clippy -D warnings` as the lint gate (the prototype is clippy-clean
but CI does not enforce it), a pinned `rust-toolchain.toml` used by every
workflow, and a JUnit summary (`cargo nextest`). `rustfmt` is a fresh choice:
the "don't reformat" constraint does not apply to new code.
