---
name: Experiment Catalog
description: Index of hypothesis-driven development experiments
type: index
source: human
---

# Experiments

Hypothesis-driven development with evidence logging. Each experiment tests a falsifiable claim and records the outcome.

## How to Run an Experiment

Use `/experiment` skill:
- `/experiment` — start a new experiment
- `/experiment log` — show the log
- `/experiment <id>` — resume or review

## Active Experiments

| ID | Date | Hypothesis | Status |
|----|------|-----------|--------|
| [EXP-001](EXP-001.md) | 2026-04-17 | Engine single-cycle p99 < 1ms with 1 suggestor | Running |
| [EXP-005](EXP-005.md) | 2026-04-17 | Engine gracefully handles chaos: panics, malformed proposals, latency variance | Running |

## Completed Experiments

| ID | Date | Hypothesis | Outcome |
|----|------|-----------|---------|
