# 0001. Record architecture decisions, and port by them

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-12 |
| **Area** | Product |
| **Evidence** | This directory |
| **egui verdict** | Keep |
| **egui status** | N/A |

## Context

Input Viewer's design lives in four places: `.claude/CLAUDE.md`, code
comments, 117 issues and 208 merged PRs. The "why" behind a threshold is
usually in a PR body or an issue comment, and some of it has drifted: the
same fact is stated differently in the docs and the code in at least a dozen
places (see [open-questions.md](open-questions.md)).

A rewrite on Rust and egui ([0002](0002-port-to-rust-and-egui.md)) has to
decide, decision by decision, what to keep. Reading the Electron app to find
out would repeat the research every time, and would miss what is only in the
issue tracker.

## Decision

- One ADR per decision a rewrite must consciously keep, change or drop. The
  Electron app's decisions are recorded as **Accepted**, dated from the PR or
  issue that made them.
- Every ADR ends with **Porting to egui**: a verdict (Keep / Adapt / Drop /
  Decide) and where the prototype stands. The index in [README.md](README.md)
  collects those into the port's status board.
- Evidence is cited, not paraphrased from memory: issue and PR numbers,
  commits, `path:line`. Where the rationale was never written down the ADR
  says so.
- Defects and doc drift found while writing these go in
  [open-questions.md](open-questions.md), not into the ADRs as if they were
  decisions.
- An ADR is changed by superseding it, not by editing history: a new ADR, and
  the old one's status set to "Superseded by NNNN".

## Consequences

- The port has a checklist with a reason attached to every item.
- These were reconstructed in 2026-10 from code and history, not written at
  the time. Where a reason is inferred, the ADR says "inferred".
- Line numbers are from `main` at 787375b (v3.4.1) and will drift.

## Porting to egui

Keep. New egui-only decisions get their own ADRs, numbered after these.
