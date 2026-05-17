---
tags: [audit, architecture]
source: mixed
---
# KB Accuracy Rerun: Documentation vs Code

**Date:** 2026-04-11  
**Scope:** Re-check live KB pages against the current public code surface after the contract hardening  
**Status:** Current state

## Headline

The public contract docs are mostly aligned now.

This rerun found four material live-page mismatches. All four were corrected in the same session:
- transport vocabulary
- execution model
- determinism language in the philosophy pages

## Confirmed Accurate

- [Architecture/API Surfaces](/Users/kpernyer/dev/reflective/stack/bedrock-platform/converge/kb/Architecture/API%20Surfaces.md) matches the six public crates
- [Architecture/Known Drift](/Users/kpernyer/dev/reflective/stack/bedrock-platform/converge/kb/Architecture/Known%20Drift.md) matches the remaining open implementation issues
- [Concepts/Context and Facts](/Users/kpernyer/dev/reflective/stack/bedrock-platform/converge/kb/Concepts/Context%20and%20Facts.md) matches the read-only fact surface
- [Concepts/Proposals and Promotion](/Users/kpernyer/dev/reflective/stack/bedrock-platform/converge/kb/Concepts/Proposals%20and%20Promotion.md) matches the single truth pipeline
- [Building/Writing Agents](/Users/kpernyer/dev/reflective/stack/bedrock-platform/converge/kb/Building/Writing%20Agents.md) correctly teaches the `Suggestor` contract despite the legacy filename
- [Building/Crate Catalog](/Users/kpernyer/dev/reflective/stack/bedrock-platform/converge/kb/Building/Crate%20Catalog.md) matches the stable crate split

## Material Mismatches Found In This Rerun

| Page | Rerun Finding | Code Says | Status |
|---|---|---|---|
| `Architecture/Transport Protocols` | still taught `InjectFactRequest` | protocol uses `SubmitObservationRequest` | Fixed in this session |
| `Architecture/Engine Execution Model` | still taught Rayon execution and name-sorted merge order | core executes sequentially; merge order is `SuggestorId` / registration order | Fixed in this session |
| `Philosophy/Nine Axioms` | still taught parallel + name-sorted determinism story | determinism currently means stable execution with registration-order merge, not name-sorted merge | Fixed in this session |
| `Philosophy/Convergence Explained` | still taught parallel + name-sorted determinism story | same as above | Fixed in this session |

## Remaining Accuracy Gaps

### Medium

- [Hexagonal Architecture](/Users/kpernyer/dev/reflective/stack/bedrock-platform/converge/kb/Architecture/Hexagonal%20Architecture.md) still carries some old surface language (`Agent`, `ExperienceStore`, `ReadContext`-style wording) and should be updated to match `Suggestor` plus the newer store boundary split.

### Low

- [Building/Context Keys](/Users/kpernyer/dev/reflective/stack/bedrock-platform/converge/kb/Building/Context%20Keys.md) still uses `Agent` vocabulary in examples and prose.
- Some philosophy/building pages still use “agent” generically in descriptive language. That is no longer a contract bug, but it is still vocabulary drift.

## Notes

- The archived baseline reports from earlier on 2026-04-11 remain historical records and intentionally preserve the older findings.
- The live KB should now be read through this rerun plus the current pages, not through the baseline reports alone.

## Summary

The KB is no longer lying about the strict boundary. The most important public pages now line up with:
- proposal-only suggestor output
- observation-first remote vocabulary
- registration-order deterministic merge
- sequential execution in core, parallelism delegated to runtime

See also: [[Architecture/Audits/2026-04-11 Drift Rerun]], [[Architecture/Known Drift]]
