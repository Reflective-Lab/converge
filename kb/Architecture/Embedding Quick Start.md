---
tags: [architecture]
source: llm
---
# Embedding Quick Start

Concrete recipes for embedding the Converge engine. Everything here is
compiled from the actual public API — no guessing required.

## Crate Imports

```toml
# Cargo.toml
[dependencies]
converge-kernel = "3.4"     # Engine, Context (concrete), ConvergeResult
converge-pack   = "3.4"     # Suggestor, AgentEffect, ProposedFact, ContextKey, Fact
converge-model  = "3.4"     # Governed semantic types (optional)
async-trait     = "0.1"
```

```rust
// Typical import block
use converge_kernel::{Context, Engine};
use converge_pack::{
    AgentEffect, Context, ContextKey, ProposedFact, Suggestor,
};
```

**`converge-core` is not the embedding surface.** Use `converge-kernel`.

## Naming Map (Old → Current)

| Removed / Never Existed | Current API |
|---|---|
| `Agent` (trait) | `Suggestor` |
| `impl Agent for X` | `impl Suggestor for X` |
| `engine.register_in_pack(...)` | `engine.register_suggestor_in_pack(...)` |
| `ctx.seed(id, content, provenance)` | `ctx.add_input(ContextKey::Seeds, id, content)` |
| `ctx.add_fact(Fact::new(...))` | `ctx.add_input(key, id, content)` |
| `Fact::new(...)` (public constructor) | Does not exist — `Fact` is read-only outside kernel |

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

- `accepts()` must be **pure** — no side effects, no I/O.
- `execute()` is **read-only** — return an `AgentEffect`, don't mutate context.
- Idempotency is **context-based** — check `ctx.has()` / `ctx.get()`, not internal flags.
- `name()` must be unique within a run (engine sorts by name for deterministic merge).

## Seeding a Context

```rust
let mut ctx = ContextState::new();
ctx.add_input(
    ContextKey::Seeds,
    "target-company",
    serde_json::json!({ "company": "Acme Corp" }).to_string(),
)?;
```

`add_input` wraps the value in a `ProposedFact` internally and stages it
for engine validation. There is no `seed()` method.

For explicit provenance:

```rust
ctx.add_input_with_provenance(
    ContextKey::Seeds,
    "target-company",
    serde_json::json!({ "company": "Acme Corp" }).to_string(),
    "crm-executor",
)?;
```

## Registering Suggestors

```rust
let mut engine = Engine::new();

// Standalone suggestor
engine.register_suggestor(MySuggestor);

// Inside a named pack
engine.register_suggestor_in_pack("dd-pack", MySuggestor);

// Invariants (governance predicates)
engine.register_invariant(MyInvariant);
```

There is no `register_in_pack()`. The method is `register_suggestor_in_pack()`.

## Running the Engine

```rust
// Simple — run to convergence
let result = engine.run(ctx).await?;

// With HITL (human-in-the-loop) pause support
let run_result = engine.run_with_hitl(ctx).await;
```

## Reading Results

```rust
let result: ConvergeResult = engine.run(ctx).await?;

assert!(result.converged);
println!("Cycles: {}", result.cycles);
println!("Stop:   {:?}", result.stop_reason);

// Read promoted facts
for fact in result.context.get(ContextKey::Hypotheses) {
    println!("{}: {}", fact.id, fact.content);
}

// Integrity proof
println!("Clock:  {}", result.integrity.clock_time);
println!("Merkle: {}", result.integrity.merkle_root.to_hex());
```

## Integrity Proof and the Lamport Clock

Every `ConvergeResult` includes an `IntegrityProof` — a tamper-evident
summary of the convergence run:

```rust
pub struct IntegrityProof {
    pub merkle_root: MerkleRoot,  // Merkle root over all facts in deterministic order
    pub clock_time: u64,          // Final Lamport logical clock time
    pub fact_count: usize,        // Total facts in the context
}
```

The engine maintains a **Lamport logical clock** internally. It ticks on
every fact promotion, giving each fact a causal ordering without wall clocks.
Embedders don't create or tick the clock — the engine does it automatically.

What embedders **should** do with the proof:

```rust
let proof = &result.integrity;

// Verify two runs produced the same governed output
assert_eq!(proof_a.merkle_root, proof_b.merkle_root);

// Check causal depth (how many promotion events occurred)
println!("Promotions: {}", proof.clock_time);

// Audit trail: log or store the proof
log::info!(
    "Run complete: {} facts, clock={}, merkle={}",
    proof.fact_count,
    proof.clock_time,
    proof.merkle_root.to_hex(),
);
```

The Merkle root is deterministic — same suggestors + same inputs = same root.
This is how downstream systems (helms, wolfgang) can verify that a result
was produced by an unmodified convergence run.

## AgentEffect Constructors

```rust
// Nothing to contribute
AgentEffect::empty()

// Single proposal
AgentEffect::with_proposal(proposed_fact)

// Multiple proposals
AgentEffect::with_proposals(vec![fact_a, fact_b])
```

## ContextKey Variants

| Key | Purpose |
|---|---|
| `Seeds` | Initial inputs from root intent |
| `Hypotheses` | Ideas and hypotheses from analysis |
| `Strategies` | Action plans and recommendations |
| `Constraints` | Limitations and boundary conditions |
| `Signals` | Observations and environmental data |
| `Competitors` | Competitive intelligence |
| `Evaluations` | Assessments and ratings |
| `Proposals` | LLM-generated suggestions awaiting validation |
| `Diagnostic` | Errors and debug info (never blocks convergence) |

## References

- [[Architecture/API Surfaces]] — full contract inventory
- [[Architecture/Golden Path Matrix]] — which crate to reach for
- `examples/hello-convergence/src/main.rs` — working minimal example
