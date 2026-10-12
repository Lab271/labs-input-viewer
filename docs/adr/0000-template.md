# NNNN. Title in the imperative

| | |
|---|---|
| **Status** | Proposed · Accepted · Superseded by NNNN · Open |
| **Date** | YYYY-MM-DD (when the decision was made, from the PR or issue) |
| **Area** | Product · Capture · Signal · Screensavers · UI · Platform · Delivery |
| **Evidence** | #issue, PR, commit, `path:line` |
| **egui verdict** | Keep · Adapt · Drop · Decide |
| **egui status** | Done · Partial · Not started · N/A |

## Context

The problem, the forces, and the incident or measurement that settled it.
Numbers where they exist. "Rationale not recorded" where it does not --
never a reconstructed reason presented as the original one.

## Decision

What was decided, concretely enough to reimplement: thresholds, timings,
sizes, precedence.

## Consequences

What it enables, what it costs, what it constrains.

## Porting to egui

- **Verdict** -- *Keep*: same behaviour, port as is. *Adapt*: same goal,
  different mechanism. *Drop*: an Electron or browser artefact with nothing
  to carry over. *Decide*: a product question to answer before porting.
- **What carries over**, **what changes**, and where the prototype
  (`input_viewer_egui/`) stands.
