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

Do not add ambiguous helpers named as if fact content had one canonical wire
format. If a helper parses JSON, the name must say JSON and return a parse
error, not discard diagnostics as `Option`.

Current rule:

```rust
let value: MyPayload = fact.parse_json_content()?;
```

For CBOR, protobuf, MessagePack, or domain-specific payloads, callers should
use that decoder directly against `content` until there is a deliberately named
typed payload contract.

## Test Layout Rule

Rust test files live in one of two places:

- `tests/` for integration suites
- `src/` for module-owned unit/property tests, and they must be explicitly module-linked

Do not leave live-looking Rust tests in ad hoc directories. They will rot and silently stop running.

`just test-layout` exists to catch the directory mistake. Feature-gated suites still need explicit run commands.

## Current Non-Default Coverage

The notable remaining property suite that is real but not default is:

- `crates/runtime/tests/wasm_property_tests.rs`

It only runs with `converge-runtime/wasm-runtime`.

Use:

```bash
just test-runtime-wasm
```

## Remaining Cleanup

The type direction is clear, but not complete. Remaining range-heavy or clamp-heavy surfaces include:

- kernel-boundary proposal confidence fields in `converge-core`
- formation confidence fields in `converge-core`
- evaluation scores in `converge-core`
- gate severity helpers in `converge-pack`
- provider-side quality thresholds and similar `0..1` values

The rule stays the same: if the domain owns the range, the type should own the range.

See also: [[Concepts/Proposals and Promotion]], [[Architecture/Purity Rules]], [[Architecture/System Overview]]
