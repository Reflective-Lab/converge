---
tags: [architecture, contract]
source: mixed
---
# Suggestor Contract

The universal loop contract. Every participant in convergence implements this
trait: domain logic, policy gates, optimization solvers, analytics, knowledge,
and custom LLM agents alike.

## The Trait

```rust
#[async_trait::async_trait]
pub trait Suggestor: Send + Sync {
    fn name(&self) -> &str;
    fn dependencies(&self) -> &[ContextKey];
    fn accepts(&self, ctx: &dyn Context) -> bool;
    async fn execute(&self, ctx: &dyn Context) -> AgentEffect;
}
```

## Method Contract

| Method | Contract |
|---|---|
| `name()` | Unique within a run. Used for logging and provenance, not sequencing |
| `dependencies()` | Context keys the suggestor reacts to |
| `accepts()` | Pure, deterministic eligibility predicate |
| `execute()` | Read-only execution that returns proposals via `AgentEffect` |

## What a Suggestor Sees

`&dyn Context` exposes the current promoted context:

- `ctx.has(key)`
- `ctx.get(key)`
- `ctx.count(key)`

A suggestor does not see the engine internals, other suggestors, or mutable
state. Context is the only shared state.

## What a Suggestor Returns

`AgentEffect`:

- `AgentEffect::empty()`
- `AgentEffect::with_proposal(...)`
- `AgentEffect::with_proposals(...)`

Those proposals are `ProposedFact`, not `Fact`. Promotion is the engine's job.

## Heterogeneous Agents, Same Loop

| Agent type | Example |
|---|---|
| Domain logic | `InvoiceCreatorAgent`, `RiskAssessorAgent` |
| Policy | `PolicyGateSuggestor`, `FlowGateSuggestor` |
| Optimization | `SolverSuggestor<BudgetAllocationPack>` |
| Analytics | `FeatureAgent`, `ModelTrainingAgent` |
| Knowledge | `KnowledgeRetrievalSuggestor`, `KnowledgeStoreSuggestor` |
| LLM reasoning | any custom `Suggestor` that calls a model provider |

There are no side-car pipeline traits for these roles. Registration still goes
through `register_suggestor()` or `register_suggestor_in_pack()`.

## Orchestration Rules

1. Seed context with external proposals.
2. Promote staged inputs.
3. Find eligible suggestors from dependency changes and `accepts()`.
4. Execute eligible suggestors against a read-only context snapshot.
5. Collect effects.
6. Promote proposals into facts.
7. Merge promoted facts deterministically.
8. Repeat until context stabilizes or a typed stop reason is reached.

## Ordering Rules

### Rule 1: Idempotency via Context

Never track internal `has_run` flags. Look for your own output in context:

```rust
fn accepts(&self, ctx: &dyn Context) -> bool {
    ctx.has(ContextKey::Seeds)
        && !ctx.get(ContextKey::Strategies).iter().any(|f| f.id == "my-fact-id")
}
```

### Rule 2: Same-Cycle Isolation

Eligible suggestors in the same cycle do not see each other's proposals from
that cycle. Effects are promoted and merged after execution.

### Rule 3: Sequence with Dependencies, Not Names

If B must run after A, B should depend on a key A writes.

```text
Cycle 1: Seeds written       -> planner writes Strategies
Cycle 2: Strategies changed  -> policy writes Constraints
Cycle 3: Constraints changed -> evaluator writes Evaluations
Cycle 4: Nothing changed     -> converged
```

### Rule 4: Determinism Follows Registration Order

Eligible suggestors are identified by `SuggestorId`, which is assigned at
registration time. That is the deterministic ordering mechanism. `name()` is
for humans and provenance, not hidden control flow.

See also: [[Architecture/Embedding Quick Start]], [[Concepts/Domain Packs]]
