---
tags: [concepts]
source: mixed
---
# Proposals and Promotion

This is the most important design decision in Converge: **`ProposedFact` is not `Fact`** ([[Philosophy/Nine Axioms#4. Agents Suggest, Engine Decides|Axiom 4]]).

## ProposedFact

What suggestors emit. A suggestion, not a truth.

```rust
pub struct ProposedFact {
    pub key: ContextKey,
    pub id: ProposalId,
    subject: Option<SubjectRef>,
    payload: Arc<dyn ErasedFactPayload>,
    confidence: UnitInterval,
    pub provenance: Provenance,
}
```

Every proposal carries typed payload identity, `confidence`, and `provenance`
— always. A proposal may also carry an app-owned `SubjectRef` answering what
the proposal is about. In-process payloads implement `FactPayload`; serialized
JSON appears only in `WireProposedFact` / `WireContextFact` at borders.
When a Suggestor derives a proposal from one promoted source fact, use
`ProvenanceSource::proposed_fact_for(...)` or
`ProposedFact::with_subject_from(...)` so the app-owned subject survives
without the Suggestor interpreting that subject vocabulary.

## The Promotion Gate

The live path from proposal to fact. The `PromotionGate` validates:

- **Authority** — does the proposing agent have the right to contribute to this context key?
- **Schema** — does the proposal conform to expected structure?
- **Confidence** — does the confidence meet the threshold?

```
Observation --> ProposedFact --[promotion gate]--> Fact
                                 |
                                 └--> ValidationError (rejected)
```

## Three-Tier Lifecycle

The full lifecycle in `converge-core` uses type-state patterns ([[Philosophy/Nine Axioms#5. Safety by Construction|Axiom 5]]):

1. **Observation** — raw output from a provider (LLM response, API result)
2. **Proposal<Draft>** → **Proposal<Validated>** — type-state transition enforced at compile time
3. **Fact** — promoted, governed, immutable, with a `PromotionRecord`

```rust
pub struct PromotionRecord {
    pub gate_id: GateId,
    pub policy_version_hash: ContentHash,
    pub approver: Actor,              // Human or System
    pub validation_summary: ValidationSummary,
    pub evidence_refs: Vec<EvidenceRef>,
    pub trace_link: TraceLink,
    pub promoted_at: Timestamp,
}
```

Every promoted fact carries a complete audit trail: which gate, which policy version, who approved, what evidence, and a trace link for replay.

## AgentEffect

Suggestors return proposals via `AgentEffect`:

```rust
pub struct AgentEffect {
    pub proposals: Vec<ProposedFact>,
}
```

`AgentEffect` is proposal-only. The engine collects all effects, validates them, and merges them serially in registration order by `SuggestorId` ([[Philosophy/Nine Axioms#6. Transparent Determinism|Axiom 6]]).

## App-Layer Proposal Envelopes

`ProposedFact` is the public proposal carrier. Apps that need typed dispatch
over a fixed Suggestor output vocabulary can introduce an app-layer wrapper
enum. Quorum, the first observed case, uses that wrapper to bind each output
variant to its durable outcome: chain write, cache projection, or policy gate.

```rust
// Sketch — concrete version lives in marquee-apps/quorum-sense
//   crates/quorum-app/src/lib.rs (`pub enum Proposal`)
pub enum Proposal {
    EnrichSignal { /* fields */ },     // chain write + DomainEvent
    FuzzyEvaluation { /* fields */ },  // cache projection only
    ClaimAuthority { /* fields */ },   // Arbiter Cedar gate
    // ...
}
```

The wrapper is the app's Suggestor -> Kernel admission protocol. Two
exhaustive matches over the enum — one to project each variant to a
`ProposedFact`, one to apply the post-promotion side effect — give the compiler
visibility into the app contract: every Suggestor output has a defined durable
outcome. Without a closed wrapper, admission has to recover app intent from
open payload identity such as `FactPayload::FAMILY` or payload type checks, and
the compiler cannot prove that every app-level output is handled.

Such wrappers may deliberately omit `Clone`, `Debug`, and `Serialize`. Each
variant is a one-shot admission token bound to its emitting Suggestor's
provenance ([[Philosophy/Nine Axioms#5. Safety by Construction|Axiom 5]]).
Tests that need upstream Suggestor outputs should seed them by running a
fixture `Suggestor` through the engine, not by cloning or serializing wrapper
values in setup.

This is an app-layer convention, not a platform requirement. Platform proposal
values remain ordinary values where the public carrier needs cloning,
debugging, and wire materialization. The convention is documented at the first
observed case: marquee-apps/quorum-sense — see
`kb/Architecture/Converge Usage Self-Review.md` "Proposal Envelope Doctrine."

See also: [[Concepts/Context and Facts]], [[Concepts/Agents]], [[Philosophy/Nine Axioms]]
