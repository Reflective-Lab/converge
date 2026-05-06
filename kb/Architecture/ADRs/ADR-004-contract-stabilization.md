---
source: mixed
---
# ADR-004: Contract Stabilization

## Status

Accepted

## Context

ADR-001, ADR-002, and ADR-003 defined the intended public crates, truth
pipeline, and pack authoring contract.

Those decisions have now been implemented:

- `converge-pack` is proposal-only
- authoritative `Fact` creation is kernel-only
- `converge-core::Fact` is read-only for downstream code
- `converge-client` and `converge-protocol` expose observation-first remote APIs
- the QA regression gate proves the contract at compile time and runtime

The remaining risk is not architectural ambiguity. It is regression: a future
change could accidentally widen the public surface or reintroduce authority
bypass.

## Decision

The Converge public contract is now treated as stable and strict.

### Hard rules

1. `converge-pack`, `converge-provider`, `converge-model`,
   `converge-kernel`, `converge-protocol`, and `converge-client` are the
   canonical public crates.
2. `converge-core` remains an implementation crate and must not expose
   authoritative fact construction through public re-exports.
3. `converge-pack::Fact` and `converge-core::Fact` are read-only surfaces for
   external code.
4. Kernel authority must stay behind internal construction helpers, not inherent
   public constructors.
5. The regression gate is mandatory for changes touching pack/core/protocol/
   client/runtime semantics.

### Regression gate

- `cargo test -p converge-pack --test compile_fail`
- `cargo test -p converge-core --test compile_fail --test truth_pipeline --test negative --test properties`
- `cargo test -p converge-client --test messages`

## Consequences

### Positive

- the axioms are defended by compiler checks and executable proofs
- downstream Rust code gets a narrow, idiomatic contract
- future refactors can move implementation internals without widening authority

### Negative

- public-surface changes now require deliberate semver discipline
- compile-fail tests need snapshot refreshes when the error surface changes
- Cargo feature unification means some compile-fail suites must run in isolation

## Notes

- This ADR completes the transitional work described in ADR-001 and ADR-003.
- Historical ADRs remain valid records; this ADR freezes the implemented state.
