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
    payload: Arc<dyn ErasedFactPayload>,
    confidence: UnitInterval,
    pub provenance: Provenance,
}
```

Every proposal carries typed payload identity, `confidence`, and `provenance`
— always. In-process payloads implement `FactPayload`; serialized JSON appears
only in `WireProposedFact` / `WireContextFact` at borders.

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

See also: [[Concepts/Context and Facts]], [[Concepts/Agents]], [[Philosophy/Nine Axioms]]
