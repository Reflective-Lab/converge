---
tags: [architecture, security]
source: mixed
---
# Security Review Plan

Post-restructuring security review. Run after the kernel correction is fully landed and all ADRs are implemented.

## Current Baseline

The 2026-04-11 runtime/policy review is recorded in [[Architecture/Audits/2026-04-11 Security Review]].

That review closed the immediate control-surface issues:

- unauthenticated delegation minting
- weak bearer-token fallback
- query-string token acceptance
- unwired gRPC auth
- permissive CORS defaults
- secret-bearing config logs
- unenforced HTTP body size limits

This page remains the forward plan. The audit page is the dated implementation record.

## Scope

The review targets the **six public crates** and the **kernel boundary**. The goal is to prove that the axioms are enforced by the compiler and runtime, not just by convention.

## 1. Compile-Time Enforcement (trybuild)

Prove that external code **cannot** do the following:

- Construct a `Fact` through the normal public authoring surface
- Import the old `kernel-authority` module or call public promotion constructors
- Access `ContextFact.key` directly (private field)
- Emit an `AgentEffect` with direct facts (field removed)
- Mutate a finalized `AgentEffect` instead of using `AgentEffectBuilder`
- Push facts into `ContextState` or `TrackedContext` from downstream code
- Fabricate `ContextSnapshot` internals with a struct literal
- Construct a `Proposal<Validated>` without going through `PromotionGate`
- Construct a `PromotionRecord` from outside the kernel
- Import `converge-core` internals from a pack crate

Each of these should be a `trybuild` compile-fail test with a clear error message.
v3.8 removes the public `kernel-authority` feature as the authority boundary;
Cargo feature unification is not a security boundary.

## 2. Property Tests (proptest)

Test invariants that must hold across thousands of random inputs:

- **Determinism**: same agents + same context + same registration order → same result, every time
- **Convergence termination**: engine always terminates (budget exhaustion or fixed point)
- **Append-only**: context fact count is monotonically non-decreasing within a run
- **Idempotent fixed point**: if run converges, running one more cycle produces no new facts
- **Promotion gate completeness**: every fact in the final context has a corresponding promotion event
- **No fact without proposal**: every fact traces back to a ProposedFact origin
- **Budget enforcement**: facts never exceed max_facts, cycles never exceed max_cycles

## 3. Negative Tests

Deliberately try to break the system:

### Authority Bypass
- Agent that tries to construct Fact directly (should fail to compile)
- Agent that returns a ProposedFact with confidence outside 0.0-1.0 (should be rejected)
- Agent that returns empty content (should be rejected)
- Agent that proposes to a key it doesn't declare in dependencies (verify behavior)

### Context Integrity
- Two agents proposing the same fact ID in the same cycle (conflict detection)
- Agent that mutates its own state between cycles (verify no effect on determinism)
- Context with max_facts budget of 1 (verify clean termination)
- Context with max_cycles budget of 0 (verify immediate termination)

### Promotion Gate
- Proposal with NaN confidence
- Proposal with infinite confidence
- Proposal with negative confidence
- Proposal with empty provenance
- Proposal with content that is only whitespace
- Proposal targeting ContextKey::Diagnostic (should it be special-cased?)

### HITL Gates
- Gate timeout with no reviewer (verify TimeoutAction::Reject)
- Gate approval followed by immediate re-run (verify no double-promotion)
- Gate rejection followed by re-proposal in next cycle
- Concurrent HITL pause and budget exhaustion

### Engine Edge Cases
- Zero agents registered (should converge immediately)
- Agent that always proposes (verify budget termination)
- Agent whose accepts() panics (verify engine doesn't crash)
- Agent whose execute() panics (verify engine doesn't crash)
- 1000 agents in parallel (verify no race conditions in merge)

## 4. API Surface Audit

For each public crate, verify:

- Every public type, trait, and function is intentional
- No internal types leak through re-exports
- No `pub` that should be `pub(crate)`
- No `unsafe` anywhere (grep verification)
- No `SystemTime::now()` in kernel crates (after clock injection)

## 5. Wire Protocol Security

- Malformed protobuf messages (fuzz with arbitrary bytes)
- Oversized messages (verify limits)
- Missing required fields in requests
- Invalid sequence numbers in resume
- Unauthenticated stream connections (when auth is enabled)
- Rapid reconnection (verify no state corruption)

## 6. Dependency Audit

```bash
just sec-deny        # cargo deny check (supply chain)
just security-audit  # blocking release-candidate dependency audit
cargo geiger         # unsafe usage in dependency tree
```

## Regression Gate

Minimum regression gate for changes touching runtime, policy, auth, or protocol:

```bash
just security-gate
```

Current command set:

```bash
cargo check --workspace
cargo test -p converge-runtime --lib
cargo test -p converge-pack --test compile_fail
cargo test -p converge-core --test compile_fail --test truth_pipeline --test negative --test properties
cargo test -p converge-client --test messages
```

## Execution Order

1. Compile-time enforcement tests (trybuild) — these are the most important
2. Property tests (proptest) — prove invariants at scale
3. Negative tests — explore edge cases
4. API surface audit — verify no leaks
5. Wire protocol tests — fuzz and malform
6. Dependency audit — supply chain

## Exit Criteria

- All trybuild tests pass (external code cannot forge facts)
- All property tests pass across 10,000+ iterations
- All negative tests pass (system rejects invalid input gracefully)
- Zero first-party `unsafe` blocks in the Converge workspace
- Zero `SystemTime::now()` in kernel crates
- `cargo deny check` clean
- `cargo audit` clean or carrying only documented, time-boxed exceptions

See also: [[Philosophy/Nine Axioms]], [[Architecture/Known Drift]], [[Architecture/API Surfaces]]
