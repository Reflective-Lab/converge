---
tags: [architecture]
source: mixed
---
# Embedding Quick Start

Concrete recipes for embedding the Converge engine with the current public API.

## Crate Imports

```toml
[dependencies]
converge-kernel = "3"
converge-model = "3"   # optional curated semantic types
async-trait = "0.1"
```

Most embedders can import directly from `converge-kernel`, which re-exports
the pack-facing traits they need:

```rust
use converge_kernel::{
    AgentEffect, Context, ContextKey, ContextState, Engine, ProposedFact, Suggestor,
};
```

Use `converge-pack` directly when you are authoring reusable pack crates. Use
`converge-kernel` when you are embedding and running the engine.

## Naming Map

| Removed / stale name | Current API |
|---|---|
| `ContextView` | `Context` |
| `Context::new()` | `ContextState::new()` |
| `register_in_pack(...)` | `register_suggestor_in_pack(...)` |
| `Agent` trait | `Suggestor` |
| public `Fact::new(...)` | not available outside kernel-authority code |

## Implementing a Suggestor

```rust
struct MySuggestor;

#[async_trait::async_trait]
impl Suggestor for MySuggestor {
    fn name(&self) -> &str {
        "my-suggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Seeds) && !ctx.has(ContextKey::Hypotheses)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let input = &seeds[0].content;

        AgentEffect::with_proposal(
            ProposedFact::new(
                ContextKey::Hypotheses,
                "my-hypothesis-1",
                format!("Analysis of: {input}"),
                "my-suggestor",
            )
            .with_confidence(0.9),
        )
    }
}
```

### Rules

- `accepts()` is pure. No I/O. No state mutation.
- `execute()` is read-only. Return an `AgentEffect`.
- `name()` must be unique within a run, but it is **not** a sequencing tool.
- Deterministic ordering follows registration order via `SuggestorId`.
- If you need agent B after agent A, model that with dependency keys, not names.

## Seeding a Context

```rust
let mut ctx = ContextState::new();
ctx.add_input(
    ContextKey::Seeds,
    "target-company",
    serde_json::json!({ "company": "Acme Corp" }).to_string(),
)?;
```

For explicit provenance:

```rust
ctx.add_input_with_provenance(
    ContextKey::Seeds,
    "target-company",
    serde_json::json!({ "company": "Acme Corp" }).to_string(),
    "crm-executor",
)?;
```

External input enters as proposals and is still promoted by the engine. That is
the intended path.

## Registering Suggestors

```rust
let mut engine = Engine::new();

engine.register_suggestor(MySuggestor);
engine.register_suggestor_in_pack("analysis", AnotherSuggestor);
engine.register_invariant(MyInvariant);
```

Those are the only Suggestor registration entry points.

## Mixing Heterogeneous Suggestors

One loop can mix many kinds of suggestors:

```rust
engine.register_suggestor(MySuggestor);
engine.register_suggestor(converge_optimization::suggestor::SolverSuggestor::budget_allocation());
engine.register_suggestor(policy_gate);
engine.register_suggestor(knowledge_retrieval);
```

Policy, optimization, analytics, knowledge, and custom LLM agents all enter
through `Suggestor`.

## Running the Engine

```rust
let result = engine.run(ctx).await?;
assert!(result.converged);
```

Advanced embedders can also use `run_with_hitl(...)` or the typed intent path.

## Reading Results

```rust
println!("Cycles: {}", result.cycles);
println!("Stop:   {:?}", result.stop_reason);

for fact in result.context.get(ContextKey::Hypotheses) {
    println!("{}: {}", fact.id, fact.content);
}

println!("Clock:  {}", result.integrity.clock_time);
println!("Merkle: {}", result.integrity.merkle_root.to_hex());
```

## Fact Authority Boundary

- `ProposedFact` is constructible by consumers and suggestors.
- `Fact` is readable by consumers.
- Authoritative `Fact` construction is gated behind `kernel-authority`.
- The engine promotion path is the normal route from proposal to fact.

See also: [[Architecture/API Surfaces]], [[Architecture/Suggestor Contract]]
