---
name: Experiment Log
description: Mutation log of all hypothesis-driven experiments
type: log
source: human
---

# Experiment Log

Record of all experiments run. Each row links to the experiment file and records the outcome.

| ID | Date | Hypothesis (short) | Status | Outcome |
|----|------|---------------------|--------|---------|
| [EXP-001](EXP-001.md) | 2026-04-17 | Single-cycle latency p99 < 1ms | running | pending |
| [EXP-002](EXP-002.md) | 2026-04-17 | Context ID validation gaps (typed-ID ADR evidence) | running | pending |
| [EXP-004](EXP-004.md) | 2026-04-17 | Memory stability: RSS growth < 50% per 100 iterations | running | pending |
| [EXP-005](EXP-005.md) | 2026-04-17 | Resilience under chaos | running | pending |
| [EXP-006](EXP-006.md) | 2026-04-21 | 6 canonical contracts sufficient for all downstream layers | confirmed (falsified) | Kernel boundary holds; 6 missing public APIs block Axiom/Wolfgang/Helms/Hackathon migration |
