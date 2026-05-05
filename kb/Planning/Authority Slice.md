---
tags: [planning, release, architecture, blocker, organism]
source: llm
date: 2026-05-05
---
# Authority Slice — v3.8 Blocker for Organism

A focused Converge slice that must land before Organism builds its Truth Document → IntentPacket bridge. ADR-006 replaces the former `kernel-authority` Cargo feature with a real boundary so Organism's bridge is not born deprecated.

See [[ADR-006-promotion-authority-boundary]] and [[v3.8 Foundation]].

## Why this gates Organism

- Organism has zero `kernel-authority` imports today (verified). The exposure is forward-looking.
- The Truth bridge and `Runtime.resolve_and_admit_truth()` are the headline E2 signals for Organism's next release. Both are Fact-adjacent. They must target admission and snapshot APIs, not raw construction.
- Production exposure today: `helms/crates/application-storage/src/lib.rs:13` (`new_fact_with_promotion`). Research exposure: 4 call sites in `engagements/newspaper`.

**Rule:** do not let Organism build against `kernel-authority` or raw `Fact` construction. The bridge targets the stable admission API; persistence targets the verified snapshot API.

## Scope

### Boundary mechanism

ADR-006 chooses the private `converge-core` promotion module. The slice does not
create a separate authority crate and does not use a sealed witness across
`converge-pack` and `converge-core`.

`converge-pack` keeps the authoring surface: `AgentEffect`, `ProposedFact`,
`Context`, and read-only context projections. Authoritative promoted state is
engine-owned and constructed only by `converge-core` promotion/admission
machinery.

### 1. Organism-facing admission API

- **Public entrypoint:** `converge_kernel::admission::admit_observation`.
- **Request type:** `AdmissionRequest`.
- **Required typed metadata:** `AdmissionActor`, `AdmissionActorKind`,
  `AdmissionSource`, `AdmissionContent`, `ContextKey`, `ProposalId`.
- **Optional semantic target:** `TruthId`.
- **Output:** `AdmissionReceipt` with context key, staged proposal id, content
  hash, target truth id, and staged/idempotent status.
- **Implementation location:** `converge-core::ContextState::submit_observation`
  stages a `ProposedFact`; `converge-kernel` exposes the supported embedding
  facade; `converge-model` re-exports the semantic request and receipt types.
- **Not exposed:** `Fact` constructor.
- **Deferred:** promoted fact references and final admission resolution. The
  first slice only proves external observations enter the promotion pipeline
  without bypassing it.

### 2. Three-concept type split

- `ProposedFact` — suggestor output.
- `ContextFact` / `FactRecord` — read-only public projection.
- Internal promoted fact state — only Converge promotion/admission can construct.

### 3. Remove `kernel-authority` as the public authority mechanism

- **Done for the v3.8 slice.** `converge-pack` no longer exposes
  `kernel-authority` as a feature or module.
- Authoritative construction lives behind the engine/kernel admission path.
- Cargo feature unification cannot make authority constructors visible to
  downstream crates.

### 4. Helms rehydration path

- **Supported API:** `ContextState::snapshot()` produces a `ContextSnapshot`;
  `ContextState::from_snapshot(snapshot)` rehydrates after validation.
- Validation rejects key mismatches, duplicate fact/proposal IDs, and Merkle
  root mismatches.
- Rehydration is distinct from creating new truth. Storage persists the
  Converge-owned snapshot envelope; it does not import `kernel_authority`, call
  fact constructors, or rebuild promotion metadata by hand.
- `helms/crates/application-storage` is the production migration target:
  replace its `PersistedFact::into_runtime` / `new_fact_with_promotion` path
  with snapshot persistence.

### 5. Compile-fail proofs

- **Done:** downstream cannot build `AgentEffect` with facts through a public
  `facts` field.
- **Done:** downstream cannot mutate the proposal vector directly through a
  public `proposals` field.
- **Done:** downstream cannot import `converge_pack::fact::kernel_authority`.
- **Done:** downstream cannot construct an authoritative fact through the old
  `Fact::construct` path or convert `ProposedFact` into `ContextFact`.
- **Done:** downstream cannot bypass admission by pushing facts into
  `ContextState` or `TrackedContext`.
- **Done:** downstream cannot fabricate `ContextSnapshot` internals with a
  struct literal.

### 6. Runtime negative tests

- Admission without actor → rejected.
- Admission without provenance → rejected.
- Forged / mismatched content hash → rejected.
- Malformed restored snapshot → rejected.
- Duplicate admission is deterministic — either idempotent or explicit conflict.

### 7. Organism-shaped compile-pass example

- Truth Document / IntentPacket-style input enters Converge through admission.
- Compiles using only supported public surfaces.
- Does not import `kernel-authority`, `converge-core` internals, or raw constructors.

## Sequencing

```
ADR-006 mechanism decision
   → admission API
   → Helms storage migration path
   → compile-fail proofs
   → Organism-shaped compile-pass proof
   → THEN Organism Truth bridge
```

Provider / Analytics / Policy / Knowledge extraction (the rest of v3.8) can proceed in parallel planning. They do not block Organism. The Authority Slice does.

As of the v3.8 authority slice, the Converge side has the admission API,
read-only `ContextFact` projection, verified `ContextSnapshot` rehydration
boundary, and compile-fail proofs. Helms still needs the downstream migration
from custom persisted facts to `ContextSnapshot`.

## Out of scope

- Rebuilding the existing kernel-authority callers in `engagements/newspaper` — research-grade, can wait for the new boundary.
- Cedar policy work beyond what's already in flight.
- Anything in [[v3.8 Foundation]] sections beyond authority and admission.
