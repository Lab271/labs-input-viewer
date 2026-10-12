# 0050. Supply chain and licence: SHA-pinned actions, delayed Dependabot, public registries, Apache-2.0 with SPDX headers

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-04-16 (75c56d7, #34, SHA pins); 2026-07-22 (604c8be Apache-2.0, 45346aa SPDX); 2026-07-30 (b819078 Dependabot, SECURITY.md, NOTICE removed) |
| **Area** | Delivery |
| **Evidence** | `.github/dependabot.yml`; `.github/workflows/*`; `LICENSE`; `SECURITY.md`; `CODE_OF_CONDUCT.md`; `~/Developer/CLAUDE.md` (org rules) |
| **egui verdict** | Keep |
| **egui status** | Partial |

## Decision

- Every GitHub Action pinned to a full commit SHA (org rule).
- Dependabot for npm and GitHub Actions, weekly, with a 7-day cooldown --
  "a public repo publishes its dependency tree". Dependabot security updates,
  secret scanning and push protection on.
- Packages from public registries only; there has never been a JFrog
  registry in this repo.
- Relicensed MIT -> Apache-2.0. Every source file carries
  `SPDX-License-Identifier: Apache-2.0` and
  `SPDX-FileCopyrightText: 2025-2026 Schuberg Philis / Lab271`.
- **NOTICE deliberately removed:** Apache-2.0 section 4(d) would oblige every
  fork to carry a NOTICE that adds nothing.
- SECURITY.md reports to abuse@schubergphilis.com and states the code is
  AI-written and unaudited.

## Consequences

- Trailing version comments on pinned SHAs go stale (`actions/checkout@... #
  v4` is actually v7.0.1).
- The removed NOTICE conflicts with the org's OSS release check, which
  expects one.

## Porting to egui

**Keep.** Prototype: actions SHA-pinned (including `actions/setup-dotnet`);
SPDX headers on every `.rs` file; `license = "Apache-2.0"` in `Cargo.toml`;
crates from crates.io. Missing: the `cargo` ecosystem in `dependabot.yml`,
and `cargo-deny` or `cargo-audit` in CI.
