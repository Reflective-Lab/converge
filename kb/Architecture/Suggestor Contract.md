---
tags: [architecture, contract]
source: mixed
---
# Suggestor Contract

The universal agent interface. Every agent in the system — LLM, optimizer,
policy gate, ML pipeline, domain logic — implements this one trait.

## The Trait

```rust
#[async_trait]
pub trait Suggestor: Send + Sync {
    fn name(&self) -> &str;
    fn dependencies(&self) -> &[ContextKey];
    fn accepts(&self, ctx: &dyn Context) -> bool;
    async fn execute(&self, ctx: &dyn Context) -> AgentEffect;
}
```

## What Each Method Does

| Method | Contract |
|--------|----------|
| `name()` | Unique within a run. Used for ordering, logging, provenance |
| `dependencies()` | Context keys this agent reads. Engine uses this for eligibility |
| `accepts()` | Pure predicate. Must be deterministic. Check idempotency via context |
| `execute()` | Produce effects. Read-only — never mutate context, return proposals |

## What a Suggestor Receives

`&dyn Context` — a read-only view of the current state:
- `ctx.has(key)` — does this key have facts?
- `ctx.get(key)` — get all facts for a key
- `ctx.count(key)` — number of facts in a key

The Suggestor cannot see other agents, cannot mutate state, cannot
access the engine. It sees only the current context snapshot.

## What a Suggestor Returns

`AgentEffect` — proposals only:
- `AgentEffect::empty()` — no contribution this cycle
- `AgentEffect::with_proposal(proposed_fact)` — single proposal
- `AgentEffect::with_proposals(vec![...])` — multiple proposals

Proposals are `ProposedFact` — not `Fact`. Promotion happens through the
engine's validation gate. A Suggestor never produces authoritative facts.

## How the Engine Orchestrates

1. Seed context with initial proposals
2. Promote seed proposals (first cycle)
3. Check which agents are eligible (dependencies modified since last cycle)
4. Run eligible agents concurrently (future: parallel)
5. Collect effects, merge proposals
6. Validate and promote proposals
7. If context changed → repeat from step 3
8. If context stable → converged (fixed point)

Budget limits cycles. If exceeded before convergence, the engine stops
with `StopReason::BudgetExhausted`.

## Heterogeneous Agents — Same Contract

| Agent Type | Example | Still a Suggestor |
|-----------|---------|-------------------|
| LLM reasoning | Claude-based planner | Yes — calls API in execute() |
| Optimization solver | OR-Tools budget allocation | Yes — runs solver in execute() |
| Policy gate | Cedar-based FlowGate | Yes — evaluates policy in execute() |
| ML inference | Feature engineering agent | Yes — runs Polars in execute() |
| Domain logic | Invoice creator, risk assessor | Yes — pure business logic |
| Adversarial | Skeptic agent | Yes — challenges plans via Constraints |

All participate through `register_suggestor()`. Same governance, same
convergence loop, same ExperienceStore capture.

## Convergence Ordering Rules

Agents within a cycle execute sorted by name. This creates implicit ordering.
Understanding this prevents subtle bugs.

### Rule 1: Idempotency via Context

Never use internal state to prevent re-firing. Always check context:

```rust
fn accepts(&self, ctx: &dyn Context) -> bool {
    // Good: check for MY output in context
    ctx.has(ContextKey::Seeds)
        && !ctx.get(ContextKey::Strategies).iter().any(|f| f.id == "my-fact-id")
}
```

### Rule 2: Same-Cycle Visibility

Agents eligible in the same cycle do NOT see each other's proposals from
that cycle. Proposals are collected, then promoted AFTER all agents run.

If agent A and agent B both depend on `ContextKey::Strategies`:
- Both fire in the same cycle when Strategies changes
- A cannot see B's proposals from this cycle (and vice versa)
- Both see the promoted results in the NEXT cycle

### Rule 3: Dependency-Driven Sequencing

To ensure agent B runs AFTER agent A:
- Make B depend on A's OUTPUT key (e.g., `ContextKey::Constraints`)
- B becomes eligible only when that key is modified
- This creates a natural multi-cycle pipeline

```
Cycle 1: Seeds written → Planner fires (writes Strategies)
Cycle 2: Strategies written → Policy fires (writes Constraints)
Cycle 3: Constraints written → Evaluator fires (writes Evaluations)
Cycle 4: Nothing changed → converged
```

### Rule 4: Name Ordering Within a Cycle

When multiple agents share the same dependencies and fire in the same cycle,
they execute sorted by `name()`. Use this for determinism, not for sequencing.
If you need guaranteed ordering, use different dependency keys.

See also: [[Formation Pattern]], [[Hexagonal Architecture]]
