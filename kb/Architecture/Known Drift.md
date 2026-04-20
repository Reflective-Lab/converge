---
tags: [architecture, philosophy]
source: mixed
---
# Known Drift

This page tracks where the implementation diverges from the [[Philosophy/Nine Axioms|axioms]]. These are not bugs to hide — they are debts to pay.

## Resolved

### Two Fact Models (Axiom 4) — CLOSED
Single truth pipeline enforced. `AgentEffect` is proposal-only. `Fact` has no public constructor. Engine promotes through `PromotionGate`. 26 tests prove the contract. Compile-fail tests prove external code cannot forge facts.

### Crate Boundary Split — CLOSED
Split into `converge-pack` (authoring) and `converge-provider-api` (capability routing). The old compatibility facade is gone from the current workspace.

### Provenance Gap — CLOSED
`Fact` now carries read-only promotion metadata (actor, validation summary, evidence refs, trace link, timestamps). The engine projects governed facts with full audit trail into the public type.

### Protocol Vocabulary — CLOSED
`InjectFactRequest` replaced by `SubmitObservationRequest`. Wire vocabulary matches the truth pipeline.

### Fact::new Re-export Gap — CLOSED
`Fact::new()` and `Fact::with_promotion()` are `kernel-authority` gated and not re-exported through `converge-core`. Compile-fail test proves external converge-core consumers cannot construct facts.

### Warning Drift — CLOSED
`cargo check --workspace` is clean (zero warnings).

### Merge Order Documentation (Axiom 6) — CLOSED
Live docs now match the implementation: core merges in registration order by `SuggestorId`.

## Open

### Medium: Structured Output Is Format-Enforced, Not Schema-Enforced

The provider boundary now validates `Json`, `Yaml`, and `Toml` responses and rejects prose wrappers with `LlmError::ResponseFormatMismatch`. But it still does not enforce an exact output schema such as required keys, field types, enum values, or nested object shape.

**Resolution:** Add a schema-aware structured output surface above `ResponseFormat` rather than overloading the format enum with shape semantics.

### Medium: SystemTime in Core (Axiom 6)

Core still sources wall-clock time internally through both `SystemTime::now()` and `Timestamp::now()`. This breaks replay determinism.

**Resolution:** Replace with an injectable `Clock` trait at the kernel/application boundary.

### Low: RetryPolicy in Core (Axiom 8)

`RetryPolicy` with `jitter_percent` lives in `converge-core`. This is an execution concern.

**Resolution:** Move to `converge-provider` or `converge-provider-api`.

### Low: ortools-sys unsafe (Axiom 5)

29 `unsafe` blocks for FFI behind the `link` feature.

**Resolution:** Per-crate lint override with explanation.

See also: [[Philosophy/Nine Axioms]], [[Architecture/API Surfaces]]
