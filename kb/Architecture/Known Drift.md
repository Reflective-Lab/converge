---
tags: [architecture, philosophy]
source: mixed
---
# Known Drift

This page tracks where the implementation diverges from the [[Philosophy/Nine Axioms|axioms]]. These are not bugs to hide — they are debts to pay.

## Resolved

### Two Fact Models (Axiom 4) — CLOSED
Single truth pipeline enforced. `AgentEffect` is proposal-only and finalized
output mutation now lives on `AgentEffectBuilder`. Engine promotes through
`PromotionGate`; compile-fail tests prove consumers cannot emit direct facts
through `AgentEffect`.

### Crate Boundary Split — CLOSED
Split into `converge-pack` (authoring) and `converge-provider-api` (capability routing). The old compatibility facade is gone from the current workspace.

### Provenance Gap — CLOSED
`Fact` now carries read-only promotion metadata (actor, validation summary, evidence refs, trace link, timestamps). The engine projects governed facts with full audit trail into the public type.

### Protocol Vocabulary — CLOSED
`InjectFactRequest` replaced by `SubmitObservationRequest`. Wire vocabulary matches the truth pipeline.

### Warning Drift — CLOSED
`cargo check --workspace` is clean (zero warnings).

### Knowledge In-Repo (ADR-008) — CLOSED
`crates/knowledge` extracted to the **mnemos** extension repo
(`~/dev/extensions/mnemos`) on 2026-05-05. Foundation no longer ships vector
storage, agentic memory, or learning implementations. Knowledge suggestors
join the convergence loop the same way as before, just from an extension
crate. See [[ADRs/ADR-008-extension-crate-boundaries]] and
[[Architecture/Plug Boundary]].

### Analytics In-Repo (ADR-008) — CLOSED
`crates/analytics` extracted to the **prism** extension repo
(`~/dev/extensions/prism`) on 2026-05-05. Foundation no longer ships
Polars/Burn-based ML pipelines, training agents, or analytics packs.
Analytics suggestors join the convergence loop from an extension crate.

### Domain Packs and Examples In-Repo — CLOSED
`crates/domain` and `examples/*` extracted to the **atelier** showcase repo
(`~/dev/atelier`) on 2026-05-05. Atelier is a single workspace gathering
worked exemplars across the Reflective Labs platform; future releases will
add demos from organism and axiom. The `analytics-packs` example was
deferred (depends on prism rewiring).

### Policy Engine In-Repo (ADR-008) — CLOSED
`crates/policy` extracted to the **arbiter** extension repo
(`~/dev/extensions/arbiter`) on 2026-05-05. Foundation `converge-pack`
keeps the policy gate trait and authorization vocabulary; Cedar engine
wiring, policy suggestors, and ed25519 delegation token verification live
in arbiter.

### Merge Order Documentation (Axiom 6) — CLOSED
Live docs now match the implementation: core merges in registration order by `SuggestorId`.

### Promotion Authority Uses a Public Cargo Feature (Axiom 4) — CLOSED

`kernel-authority` is removed from `converge-pack`. Context consumers now read
`ContextFact` projections; embedders stage external input through admission;
storage rehydrates through `ContextSnapshot`. Compile-fail tests prove
downstream code cannot import `kernel_authority`, push facts into
`ContextState`/`TrackedContext`, convert `ProposedFact` into `ContextFact`, or
fabricate snapshot internals with a struct literal.

**Remaining downstream migration:** Helms `application-storage` still needs to
replace its custom persisted-fact reconstruction with `ContextSnapshot`.

## Open

### High: Provider Contract Naming Is Backwards

The stable provider contract is named `converge-provider-api`, while the
implementation crate owns the clean `converge-provider` name. That inverts the
long-term naming rule: contracts get domain names; implementations add adapter
or implementation qualifiers.

**Resolution:** Decide the v3.8 extraction path for provider/tool contracts and
adapters. Treat the current `converge-provider-api` name as transitional.

### High: External Tool and Provider Implementations Still Sit In-Repo

HTTP fetch, feed parsing, search providers, LLM adapters, API-key handling, and
third-party SDK churn still live beside the Converge kernel workspace. This
keeps volatile external I/O too close to the correctness-first core.

**Resolution:** Move provider and external tool implementations to adapter
crates or a separate repository. Keep only stable port contracts in the
Converge foundation.

### High: Extension Implementations Share the Foundation Release Cycle

Provider, analytics, policy, and knowledge implementation crates still compile
and release with the foundation repository. They are useful implementations,
but they carry heavy dependencies, external service churn, and different
release pressure than the kernel and public contracts.

**Resolution:** Keep the small traits, semantic contracts, and boundary DTOs in
Converge. Move implementation-heavy crates to extension crates or external
repositories after ADR-008 classifies each boundary.

### High: Organism Truth Bridge Could Be Born-Deprecated

Organism currently has no `kernel-authority` imports, and Converge now has the
v3.8 admission/snapshot boundary. The risk is downstream sequencing: if
Organism builds Fact-adjacent bridge code against anything other than
admission and snapshot APIs, it will create fresh drift.

**Resolution:** Organism Truth bridge work targets
`converge_kernel::admission` and never raw fact construction. Treat Helms
`application-storage` as the production snapshot migration surface and
newspaper engagement call sites as lower-priority research exposure.

### Medium: Structured Output Is Format-Enforced, Not Schema-Enforced

The provider boundary now validates `Json`, `Yaml`, and `Toml` responses and rejects prose wrappers with `LlmError::ResponseFormatMismatch`. But it still does not enforce an exact output schema such as required keys, field types, enum values, or nested object shape.

**Resolution:** Add a schema-aware structured output surface above `ResponseFormat` rather than overloading the format enum with shape semantics.

### Medium: SystemTime in Core (Axiom 6)

Core still sources wall-clock time internally through both `SystemTime::now()` and `Timestamp::now()`. This breaks replay determinism.

**Resolution:** Replace with an injectable `Clock` trait at the kernel/application boundary.

### Low: RetryPolicy in Core (Axiom 8)

`RetryPolicy` with `jitter_percent` lives in `converge-core`. This is an execution concern.

**Resolution:** Move to `converge-provider` or `converge-provider-api`.

See also: [[Philosophy/Nine Axioms]], [[Architecture/API Surfaces]]
