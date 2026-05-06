---
tags: [audit, architecture]
source: mixed
---
# Drift Rerun: Philosophy vs Implementation

**Date:** 2026-04-11  
**Scope:** Re-run against the Nine Axioms after the contract hardening, crate split, protocol cleanup, and regression gate work  
**Status:** Current state

## Headline

No critical authority drift remains.

The enforced boundary is still intact:
- `converge-pack` is proposal-only
- authoritative `Fact` construction is kernel-only
- engine promotion goes through `PromotionGate`
- client/protocol vocabulary is observation-first
- the public contract gate is green

## Verified Clean

### Axiom 1: Explicit Authority
**Status:** Clean

Pack authors cannot construct authoritative facts. `Fact` is read-only outside kernel-authority code paths, and compile-fail tests prove downstream code cannot forge facts.

### Axiom 2: Convergence Over Control Flow
**Status:** Clean

No suggestor-to-suggestor calls, no workflow handoff model, and no hidden control channels were found in the audited code paths.

### Axiom 3: Append-Only Truth
**Status:** Clean

Facts are still append-only in context, corrections remain new entries, and conflict detection still prevents silent overwrite.

### Axiom 4: Suggestors Suggest, Engine Decides
**Status:** Clean

The previous bypass is gone. `AgentEffect` is proposal-only and the merge path promotes proposals through `PromotionGate`.

### Axiom 7: Human Authority First-Class
**Status:** Clean

HITL remains explicit in types and runtime flow. The pause/approve/reject lifecycle is still first-class.

### Hexagonal Boundaries
**Status:** Clean

The public crate split holds: `converge-pack` and `converge-provider` are leaf contracts, and the external client/protocol surfaces remain separated from core/runtime internals.

## Remaining Drift

### Medium: Axiom 6 — Transparent Determinism

Core still sources time internally.

Representative locations:
- [root_intent.rs](/Users/kpernyer/dev/work/converge/crates/core/src/root_intent.rs#L68)
- [id.rs](/Users/kpernyer/dev/work/converge/crates/core/src/types/id.rs#L347)
- [engine.rs](/Users/kpernyer/dev/work/converge/crates/core/src/engine.rs#L1414)
- [hitl.rs](/Users/kpernyer/dev/work/converge/crates/core/src/gates/hitl.rs#L231)

The issue is broader than direct `SystemTime::now()` calls. `Timestamp::now()` is also backed by wall-clock time, so replay determinism is still not fully honest.

**Resolution:** inject a clock at the kernel boundary and remove `Timestamp::now()` / `SystemTime::now()` from core-owned decision paths.

### Low: Axiom 8 — No Hidden Work

`RetryPolicy` still lives in core.

See [backend.rs](/Users/kpernyer/dev/work/converge/crates/core/src/backend.rs#L319).

This is a data type, not active retry execution, but it still teaches runtime retry semantics at the kernel layer.

**Resolution:** move retry configuration to provider/runtime surfaces.

### Low: Axiom 5 — Safety by Construction

`ortools-sys` still contains raw FFI `unsafe` blocks.

See [lib.rs](/Users/kpernyer/dev/work/converge/crates/optimization/ortools-sys/src/lib.rs#L181).

This is contained to the FFI adapter crate, but the exception should be documented explicitly rather than left implicit.

**Resolution:** add per-crate lint justification and document the FFI boundary.

## Regression Gate Rerun

The contract gate still passes:

```bash
cargo test -p converge-pack --test compile_fail
cargo test -p converge-core --test compile_fail --test truth_pipeline --test negative --test properties
cargo test -p converge-client --test messages
```

## Summary

The philosophical break is real. What remains is not authority bypass or topology confusion. The open work is:
- clock injection for determinism
- moving retry policy out of core
- documenting the OR-Tools FFI unsafe boundary

See also: [[Architecture/Known Drift]], [[Architecture/Audits/2026-04-11 KB Accuracy Rerun]]
