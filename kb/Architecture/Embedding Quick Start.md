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

## Public Surface Mantra

Keep the public placement rule simple:

- semantics in `converge-model`
- authoring in `converge-pack`
- runnable machinery in `converge-kernel`

For formations, the grouped embedder entrypoint is `converge_kernel::formation`.

## Naming Map

| Removed / stale name | Current API |
|---|---|
| `ContextView` | `Context` |
| `Context::new()` | `ContextState::new()` |
| `register_in_pack(...)` | `register_suggestor_in_pack(...)` |
| `Agent` trait | `Suggestor` |
| public `Fact::new(...)` | not available to normal authoring consumers |

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
engine.register_suggestor(optimizer);
engine.register_suggestor(policy_gate);
engine.register_suggestor(knowledge_retrieval);
```

Policy, optimization, analytics, knowledge, and custom LLM agents all enter
through `Suggestor`.

## Self-Assembling Formations

For embedders, the stable grouped API is:

```rust
use std::sync::Arc;

use converge_kernel::{
    formation::{
        Capability, DeliberatedFormationTemplate, FormationAssemblySuggestor, FormationCatalog,
        FormationRequest, FormationTemplate, FormationTemplateMetadata, FormationTemplateQuery,
        ProfileSnapshot, ProviderRequest, ProviderSelectionSuggestor, SuggestorCapability,
        SuggestorRole,
    },
    ContextKey, Engine,
};
use converge_provider_api::Backend;
```

The structured contract begins at:

- `FormationTemplateQuery`
- `FormationCatalog`
- `FormationRequest`
- `ProviderRequest`

There are two valid upstream patterns:

- structured input
  - a seeder writes those requests directly
- loose input
  - an upstream suggestor matches a `FormationTemplate` and then compiles it
    into those requests

Once the requests exist, the formation machinery is the same:

```rust
let formation_catalog = FormationCatalog::new().with_template(
    FormationTemplate::deliberated(DeliberatedFormationTemplate::new(
        FormationTemplateMetadata::new(
            "market-entry",
            "Go or no-go market entry loop",
            [SuggestorRole::Analysis, SuggestorRole::Planning],
        )
        .with_keyword("market")
        .with_keyword("launch")
        .with_entity("competitors")
        .with_required_capability(SuggestorCapability::LlmReasoning),
        3,
    )),
);

let query = FormationTemplateQuery::new()
    .with_keyword("launch")
    .with_entity("competitors")
    .with_required_capability(SuggestorCapability::LlmReasoning);

let request = formation_catalog
    .top_match(&query)
    .expect("template should exist")
    .to_request("launch-1");
```

Then assemble against the suggestor catalog for this run:

```rust,ignore
let mut engine = Engine::new();

engine.register_suggestor(FormationAssemblySuggestor::new(catalog));
engine.register_suggestor(ProviderSelectionSuggestor::new(backends));
```

`catalog` here is still a `Vec<ProfileSnapshot>`. Converge does not introspect
registered suggestors automatically; embedders or upper layers build that
catalog when they register profiled suggestors.

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
- `ContextFact` is readable by consumers as a projection.
- Authoritative fact construction is engine-owned; `kernel-authority` is no
  longer a public feature/module.
- Durable storage persists `ContextSnapshot` and rehydrates with
  `ContextState::from_snapshot`.
- The engine promotion path is the normal route from proposal to fact.

See also: [[Architecture/API Surfaces]], [[Architecture/Suggestor Contract]]
