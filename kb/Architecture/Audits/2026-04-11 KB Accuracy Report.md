---
tags: [audit, architecture]
source: mixed
---
# KB Accuracy Report: Documentation vs Code

**Date:** 2026-04-11
**Scope:** Every kb page checked against actual source code, crate by crate in dependency order
**Status:** Baseline. Re-run after kernel correction to verify fixes.

## Confirmed Accurate

- Agent trait methods (name, dependencies, accepts, execute) — correct
- Context trait methods (has, get, get_proposals, count) — correct
- ContextKey variants (all 9) — correct
- ProposedFact fields (key, id, content, confidence, provenance) — correct
- Invariant trait, InvariantClass, InvariantResult — correct
- StreamingCallback trait — correct
- Domain pack names (trust, money, delivery, knowledge, data_metrics) — correct
- Purity rules allowed/forbidden deps — correct
- Engine 8-phase model — correct
- SurrealDB, LanceDB, OR-Tools descriptions — correct
- Dependency graph and publish order — correct (updated for new crate split)

## Discrepancies Found

| # | KB Page | KB Says | Code Says | Status |
|---|---|---|---|---|
| 1 | Context and Facts | "Facts have private constructors" | pack-layer `Fact::new()` is fully public | Noted in Known Drift |
| 2 | Root Intent | `goal` field | Field is `objective: Option<Objective>` | Fixed |
| 3 | Root Intent | `Budget { cycles, facts, tokens }` | `Budget { max_cycles, max_facts }` — tokens in `Budgets` as `Option<u64>` | Fixed |
| 4 | Proposals and Promotion | `PromotionRecord { approver, approval_id, timestamp, reason }` | Actual: `{ gate_id, policy_version_hash, approver, validation_summary, evidence_refs, trace_link, promoted_at }` | Fixed |
| 5 | HITL Gates | `duration_secs`, `GateVerdict::Approved`, `reviewer` | `timeout_secs`, `GateVerdict::Approve`, `decided_by` | Pending |
| 6 | Governed Artifacts | States: Draft→UnderReview→Approved→Active→Suspended→Retired | Actual: Draft→Approved→Active→Quarantined→Deprecated/RolledBack | Pending |
| 7 | Backends | "70+ capabilities" | ~31 capability variants | Pending |
| 8 | Backends | "Firestore, PostgreSQL" | SurrealDB, PostgreSQL, SQLite | Pending |
| 9 | Proposals and Promotion | "merge in name-sorted order" | Sorts by AgentId (registration order) | Fixed |

## Missing from KB

| # | What | Status |
|---|---|---|
| 1 | `converge-storage` crate | Added to Crate Catalog |
| 2 | `converge-protocol` crate | Added to Crate Catalog |
| 3 | `converge-client` crate | Added to Crate Catalog |
| 4 | `converge-pack` crate | Added to Crate Catalog |
| 5 | `converge-provider` crate | Added to Crate Catalog |
| 6 | `converge-model` crate | Added to Crate Catalog |
| 7 | `converge-kernel` crate | Added to Crate Catalog |
| 8 | API Surfaces document | Created |
| 9 | Known Drift tracker | Created |
| 10 | 7 additional LLM providers (OpenRouter, Grok, MinMax, Baidu, Zhipu, Kimi, Kong) | Pending |
| 11 | OCR, patent, LinkedIn, Brave search providers | Pending |
| 12 | Many domain pack agents not listed | Pending |
| 13 | `ExperienceAppender`/`ExperienceReplayer` traits | Pending |
| 14 | Form filler, meeting scheduler, resource routing use cases | Pending |
| 15 | rust-version is 1.94.0, not 1.90 | Pending |

## Stale in KB

| # | What | Status |
|---|---|---|
| 1 | Governed Artifacts state machine (UnderReview, Suspended, Retired don't exist) | Pending |
| 2 | Simplified PromotionRecord struct | Fixed |
| 3 | Simplified GateDecision struct | Pending |
| 4 | Budget with `tokens` field | Fixed |
| 5 | Capability count "70+" | Pending |
| 6 | "Private constructor" claim for pack-layer Fact | Noted in Known Drift |

## Resolution Plan

Items marked "Fixed" were corrected in the kb during this session.
Items marked "Pending" will be fixed after the kernel correction lands — the code is changing, so fixing docs against a moving target creates churn.
Items marked "Noted in Known Drift" are architectural gaps tracked in [[Architecture/Known Drift]] with ADR-backed resolution plans.

The full audit should be re-run after the semantic break (ADR-002/003 implementation) to verify all findings are resolved.

See also: [[Architecture/Known Drift]], [[Architecture/Audits/2026-04-11 Drift Report]]
