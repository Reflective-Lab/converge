---
tags: [audit, architecture]
source: mixed
---
# Drift Report: Philosophy vs Implementation

**Date:** 2026-04-11
**Scope:** Full codebase audit against the Nine Axioms
**Status:** Baseline. Re-run after kernel correction to verify fixes.

> Historical note: this audit captured the pre-async kernel contract. Where this page says "no async," read it as "no runtime ownership." Current live docs allow runtime-agnostic async surfaces in `converge-core`.

## The True Story

Converge is an Agent OS where multi-agent systems produce governed, auditable, convergent outcomes. Its central insight is that agents are pure functions over a shared context: they propose evidence, and the engine decides what gets promoted to fact. No agent can unilaterally change shared state, no agent calls another agent, and all communication happens through typed context keys. The engine runs agents in cycles until a fixed point is reached or a budget is exhausted, and it tells you honestly which happened and why.

The architecture is hexagonal: converge-traits and converge-core form the pure kernel (no I/O, no runtime ownership, no non-determinism), and all external systems plug in through trait implementations in adapter crates. Facts are append-only and immutable. Proposals carry confidence and provenance. A promotion gate validates proposals before they become facts, enforcing authority, schema, and confidence requirements. Human-in-the-loop approval is a first-class concept, not a workaround.

The nine axioms are non-negotiable: explicit authority, convergence over control flow, append-only truth, agents suggest / engine decides, safety by construction, transparent determinism, human authority first-class, no hidden work, and scale by intent replication. Every design decision must serve at least one axiom and violate none.

## Axiom Compliance

### Axiom 1: Explicit Authority
**Status:** Minor Drift

The `converge-pack::Fact` (formerly traits-layer) has a fully public `Fact::new()` constructor. The governed `types::Fact` in converge-core has `pub(crate)` constructors and requires a `PromotionRecord`. The engine's merge loop at `engine.rs:778` directly adds pack-layer `Fact` objects from agent effects to the context without any promotion gate. Agents can emit facts directly, bypassing the promotion gate entirely.

### Axiom 2: Convergence Over Control Flow
**Status:** Clean

No agent-to-agent imports. No message passing, mailboxes, or channels between agents. All domain agents read from context keys and return `AgentEffect`. The engine runs cycles until fixed point.

### Axiom 3: Append-Only Truth
**Status:** Clean

No `&mut Fact` references anywhere. `Context::add_fact()` rejects modifications to existing fact IDs (returns `ConvergeError::Conflict`). No remove, delete, or mutation methods on context.

### Axiom 4: Agents Suggest, Engine Decides
**Status:** Major Drift

122 instances of `AgentEffect::with_fact()` (agents emitting facts directly) across 22 files. Zero domain agents use `AgentEffect::with_proposal()`. The `PromotionGate` in `gates/promotion.rs` is fully implemented but never invoked during the actual convergence loop. The `Fact::try_from(ProposedFact)` in the merge path does only minimal confidence/content validation — not authority, schema, or the full gate pipeline.

### Axiom 5: Safety by Construction
**Status:** Minor Drift

The type-state pattern (`Proposal<Draft>` → `Proposal<Validated>`) exists in `types/proposal.rs` but is not used by the engine or domain agents (see Axiom 4). `unsafe_code = "forbid"` is set in workspace lints. `ortools-sys` contains 29 `unsafe` blocks for FFI behind the `link` feature flag.

### Axiom 6: Transparent Determinism
**Status:** Minor Drift

Documentation states "effects merge serially in name-sorted order." The actual implementation at `engine.rs:771` sorts by `AgentId` (a monotonic `u32`), not by agent name. Deterministic only if registration order is stable.

`SystemTime::now()` is called in 6 locations within converge-core for `PromotionRecord`, `GateRequest`, `GateDecision`, and `RootIntent`. This introduces non-determinism into the audit trail.

### Axiom 7: Human Authority First-Class
**Status:** Clean

HITL gate is well-designed: `HitlPolicy` with kind/confidence/agent-based gating, `GateRequest`/`GateDecision` types, `GateEvent` audit trail, `TimeoutPolicy` with explicit timeout actions. Default is `TimeoutAction::Reject` (safe). Engine properly handles pause/resume lifecycle.

### Axiom 8: No Hidden Work
**Status:** Minor Drift

`RetryPolicy` in `backend.rs:319` defines retry logic with exponential backoff and jitter in converge-core. The "What Converge Is Not" doc lists "Add retry logic to the engine" as an anti-pattern. `RetryPolicy` is a data type not execution, but its presence in core normalizes retry semantics at the kernel level.

### Axiom 9: Scale by Intent Replication
**Status:** Clean (Not Yet Testable)

`RootIntent` and `Frame` types exist. No distributed scaling code exists yet.

## Architecture Compliance

**Purity Rules:** converge-core Cargo.toml is clean — no tokio, reqwest, axum, rand, rayon. However, `SystemTime::now()` introduces non-determinism without an external dependency.

**Hexagonal Boundaries:** No inward dependency violations. converge-pack has zero internal deps. converge-provider has zero internal deps. converge-core depends only on converge-pack. The hexagonal boundary is maintained.

**"What Converge Is Not" Patterns:** No workflow, actor, event-driven, or chatbot patterns found in core or domain code.

## Findings Summary

| Finding | Axiom | Severity | Resolution |
|---|---|---|---|
| Two competing Fact types | 4 | Critical | ADR-002: single truth pipeline |
| 122 direct fact emissions bypassing gate | 4 | Critical | ADR-003: remove AgentEffect.facts |
| Public Fact::new() constructor | 1 | Major | ADR-003: private constructor |
| PromotionGate disconnected from engine | 4 | Major | Rebuild engine merge loop |
| SystemTime::now() in core (6 locations) | 6 | Medium | Injectable Clock trait |
| Merge order: ID-sorted not name-sorted | 6 | Medium | Fix code or docs |
| RetryPolicy in core | 8 | Low | Move to provider |
| ortools-sys unsafe blocks | 5 | Low | Per-crate lint override |

See also: [[Architecture/Known Drift]], [[Philosophy/Nine Axioms]]
