---
tags: [architecture]
source: mixed
---
# Type Protocol

The rules for Converge types. This is not style. This is contract design.

## Decision Rule

When a value carries semantics, apply this order:

1. Make the invalid state unrepresentable with the type system.
2. If the value crosses an untyped boundary, validate it during construction or deserialization.
3. Only then add negative or property tests for what the type system cannot prevent.

This means:

- strings with owned vocabulary become enums
- open identifiers become newtypes
- bounded numeric domains become validated types or private validated fields
- known config schemas use strict deserialization, not permissive parse-then-validate cleanup

## What Must Be Typed

- Semantic IDs are newtypes, not raw strings.
  Examples: `FactId`, `ProposalId`, `ObservationId`, `GateId`, `PackName`, `AgentName`.
- Subjects being reasoned about use `SubjectRef`, not app URI strings in
  Converge-facing fact or proposal metadata.
- Fact and proposal payloads are typed `FactPayload` values in process. Raw
  strings are valid only for explicit text payloads or border DTOs.
- Closed vocabularies are enums, not string conventions.
  Examples: `RequirementPreset`, `Selector::Any`, typed stop reasons, owned flow/action enums.
- Bounded numbers are validated values.
  Examples: confidence, quality thresholds, scores, severity bands.
- Runtime schema identifiers and versions are typed.
  Examples: `PackName`, `PackVersion`, `VersionRequirement`, `ProviderId`.

## What Has Been Strengthened

### Kernel and Promotion Boundary

- `Fact` remains authoritative and is not part of the normal consumer surface.
- `ContextFact` is the read-only public projection returned by context reads.
- `ProposedFact` is the intended consumer write path; the engine validates and
  promotes proposals into facts.
- The old `kernel-authority` Cargo feature has been removed because public
  feature unification is not an authority boundary; see
  [[Architecture/ADRs/ADR-006-promotion-authority-boundary]].
- Durable rehydration uses `ContextSnapshot`, not raw fact constructors.
- Compile-fail tests prove the boundary from the consumer side.

### Public Semantic Surface

- Semantic IDs, hashes, timestamps, gate IDs, trace IDs, pack IDs, and policy IDs now cross the public contract as typed values instead of raw strings.
- Context naming is explicit: `converge_kernel::Context` is the trait, `ContextState` is the embedder-owned state.

### Runtime Perimeter

- Runtime auth and policy matching now separate identity from matcher semantics.
- Wildcard behavior uses `Selector<T>` instead of magic `"*"` strings in lists.
- Runtime wiring identifiers use typed values like `PackName`, `AgentName`, and `ProviderId`.

### Runtime Pack Wiring

- Pack YAML is now strict operational wiring.
- Pack names, versions, compatibility requirements, requirement presets, provider preferences, and quality thresholds are typed at deserialization time.
- Known wiring structs use `serde(deny_unknown_fields)`.
- Duplicate agent IDs are rejected during parse, not by later cleanup.
- Semantic-rule keys like `validation` and `invariants` are rejected at parse time.

### Proposal and Confidence Surface

- Proposal confidence fields are no longer casual public slots to mutate directly.
- Confidence values are accessed through methods and normalized through constructors.
- `with_confidence` and `adjust_confidence` make confidence updates explicit and clamp at the constructor boundary until a shared unit-interval type is propagated everywhere.
- `UnitInterval` is now the shared public value type for normalized confidence,
  recall scores, thresholds, and weights. Deserialization rejects NaN, infinity,
  and values outside `[0.0, 1.0]`.
- `BasisPoints` is the shared public value type for basis-point confidence and
  probability-like integer ranges (`0..=10_000`).

## Confidence API Change

The current confidence API hardening applies to these public semantic types:

- `ProposedFact`
- `ProposedContent`
- `Hypothesis`
- `ProposedPlan`

For these types:

- direct field access no longer compiles
- confidence is set through constructors and builder methods
- confidence is read through `confidence()`
- the type, not the engine, owns the `[0.0, 1.0]` invariant

Migration:

```rust
// Before
let proposal = ProposedFact { confidence: 0.8, ..todo!() };
let score = proposal.confidence;

// After
let proposal = ProposedFact::new(...).with_confidence(0.8);
let score = proposal.confidence();
```

Preferred multi-criteria pattern:

```rust
use converge_pack::{CONFIDENCE_STEP_MAJOR, CONFIDENCE_STEP_MINOR};

let proposal = ProposedFact::new(...)
    .with_confidence(0.5)
    .adjust_confidence(CONFIDENCE_STEP_MAJOR)
    .adjust_confidence(CONFIDENCE_STEP_MINOR);
```

This is the important architectural point: the engine should not be compensating for a weak proposal type. Where Converge owns the range, the type should own the range.

## Typed Fact Payloads

`converge-pack::ProposedFact` no longer carries semantic `content: String`.
The constructor requires a typed payload:

```rust
let proposal = ProposedFact::new(
    ContextKey::Seeds,
    "invoice-policy-check",
    InvoicePolicyCheckV1 { /* fields */ },
    ARBITER_PROVENANCE.provenance(),
);
```

The payload type owns the schema:

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct InvoicePolicyCheckV1 {
    decision: Decision,
}

impl FactPayload for InvoicePolicyCheckV1 {
    const FAMILY: &'static str = "arbiter.invoice-policy-check";
    const VERSION: u16 = 1;
}
```

Read typed values from context:

```rust
for fact in ctx.get(ContextKey::Decisions) {
    let payload = fact.require_payload::<InvoicePolicyCheckV1>()?;
}
```

For HTTP/gRPC, CLI fixtures, storage/replay, audit export, non-Rust clients, and
NATS/Lattice traffic, use `WireProposedFact` or `WireContextFact` plus a
`PayloadRegistry`. Unknown `(family, version)` fails closed.

Version policy is frozen tuple based: `family + v1` maps to one Rust type
forever. Schema changes create `v2`; implicit registry upgraders are forbidden.
Explicit migration suggestors may read V1 and emit V2.

See [[Standards/Typed Fact Payloads]] for the full wire and replay policy.

## Test Policy

Types do not replace every test. Keep tests where they prove something types cannot.

Keep:

- compile-fail tests for public boundary enforcement
- property tests for wire formats, serialization, determinism, and algorithmic invariants
- feature-gated tests for feature-gated code paths

Replace with stronger types when the test is mostly proving:

- a string should have been an enum or newtype
- a config validator is cleaning up after permissive deserialization
- an invalid numeric range should have been rejected at construction time
- internal misuse is possible only because fields are too public

## Content Parsing Helpers

Do not add helpers that parse semantic JSON out of `ProposedFact` or
`ContextFact`. These types no longer expose a generic semantic string payload.

If the value is meaningful inside Converge, define a `FactPayload`. If the value
is crossing a boundary, decode through the wire shape and payload registry.

## Test Layout Rule

Rust test files live in one of two places:

- `tests/` for integration suites
- `src/` for module-owned unit/property tests, and they must be explicitly module-linked

Do not leave live-looking Rust tests in ad hoc directories. They will rot and silently stop running.

`just test-layout` exists to catch the directory mistake.

## Current Non-Default Coverage

Converge no longer owns a feature-gated WASM property suite. Sandboxed
application plugins are a Helm concern, so their ABI and guest-boundary
properties live with Helm's plugin runtime rather than with `converge-runtime`.
Axiom may produce WASM artifacts and manifests, but Converge only consumes the
adapted proposal, invariant verdict, evidence refs, trace links, and typed
payloads that cross public kernel/pack contracts.

## Remaining Cleanup

The type direction is clear, but not complete. Remaining range-heavy or clamp-heavy surfaces include:

- kernel-boundary proposal confidence fields in `converge-core`
- formation confidence fields in `converge-core`
- evaluation scores in `converge-core`
- gate severity helpers in `converge-pack`
- provider-side quality thresholds and similar `0..1` values
- older type-state gate summaries that still use `ProposedContent` string text
  as a review/audit summary rather than the pack fact payload

The rule stays the same: if the domain owns the range, the type should own the range.

See also: [[Concepts/Proposals and Promotion]], [[Architecture/Purity Rules]], [[Architecture/System Overview]]
