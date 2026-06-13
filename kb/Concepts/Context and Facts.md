---
tags: [concepts]
source: mixed
---
# Context and Facts

The context is the shared, typed state visible to all suggestors. It is the only communication channel between suggestors ([[Philosophy/Nine Axioms#2. Convergence Over Control Flow|Axiom 2]]).

## Context

```rust
pub trait Context: Send + Sync {
    fn has(&self, key: ContextKey) -> bool;
    fn get(&self, key: ContextKey) -> &[ContextFact];
    fn get_proposals(&self, key: ContextKey) -> &[ProposedFact];
    fn count(&self, key: ContextKey) -> usize;
}
```

Properties:
- **Append-only** — facts are added, never removed or modified ([[Philosophy/Nine Axioms#3. Append-Only Truth|Axiom 3]])
- **Partitioned** — facts are organized by [[Building/Context Keys|ContextKey]]
- **Read-only for suggestors** — suggestors receive `&dyn Context`, not a mutable reference
- **Scoped to a run** — context exists for one convergence run, not across runs

## Context Facts

A `ContextFact` is the read-only public projection of a promoted fact in the context.

```rust
impl ContextFact {
    pub fn key(&self) -> ContextKey;
    pub fn id(&self) -> &FactId;
    pub fn subject(&self) -> Option<&SubjectRef>;
    pub fn payload<T: FactPayload>(&self) -> Option<&T>;
    pub fn promotion_record(&self) -> &FactPromotionRecord;
    pub fn created_at(&self) -> &Timestamp;
}
```

Context facts are authoritative and read-only to consumers. Suggestors emit
`ProposedFact`; the engine validates through the promotion gate, and only then
does a projected `ContextFact` enter context. Facts and proposals may be
tagged with an app-owned `SubjectRef`, so embedders can query
`ContextState::facts_for_subject` and `proposals_for_subject` without parsing
app URI strings or payload internals. Durable storage uses `ContextSnapshot`
for rehydration; it does not call fact constructors. Generic pack execution
follows the same rule: `PackSuggestor` preserves a subject from the input
`PackInputPayload` fact on the emitted `PackPlanPayload` proposal.

## Context Keys

Facts are partitioned by `ContextKey`. Suggestors declare which keys they depend on — the engine only wakes suggestors when their dependencies change.

| Key | Purpose |
|---|---|
| `Seeds` | Initial evidence, raw inputs |
| `Hypotheses` | Tentative conclusions |
| `Strategies` | Action plans and recommendations |
| `Constraints` | Limitations and boundary conditions |
| `Signals` | Observations, environmental data |
| `Competitors` | Competitive intelligence |
| `Evaluations` | Assessments, scores, rankings |
| `Proposals` | LLM-generated suggestions awaiting validation |
| `Diagnostic` | Debugging info (never blocks convergence) |

See also: [[Concepts/Proposals and Promotion]], [[Building/Context Keys]], [[Philosophy/Nine Axioms]]
