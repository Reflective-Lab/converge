---
tags: [building]
source: mixed
---
# Getting Started

## Add Converge to Your Project

```toml
[dependencies]
converge-kernel = "3"
```

That's enough to embed the Converge engine in-process.

Need more?

```toml
converge-pack = "3"          # Author suggestors and invariants
converge-model = "3"         # Curated semantic types
converge-domain = "3"        # Pre-built domain packs
converge-client = "3"        # Remote Rust client
async-trait = "0.1"          # Implement async Suggestor
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }  # or use your runtime
```

See [[Building/Crate Catalog]] for the full list.

## First Run

```rust
use async_trait::async_trait;
use converge_kernel::{
    AgentEffect, Context, ContextKey, Context, Engine, ProposedFact, Suggestor,
};

#[tokio::main]
async fn main() {
    struct SeedSuggestor;

    #[async_trait]
    impl Suggestor for SeedSuggestor {
        fn name(&self) -> &str { "seed" }
        fn dependencies(&self) -> &[ContextKey] { &[] }
        fn accepts(&self, ctx: &dyn Context) -> bool {
            !ctx.has(ContextKey::Seeds)
        }
        async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
            AgentEffect::with_proposal(ProposedFact {
                key: ContextKey::Seeds,
                id: "observation-1".into(),
                content: "Monthly active users grew 15%".into(),
                confidence: 0.95,
                provenance: "suggestor:seed".into(),
            })
        }
    }

    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor);
    let result = engine.run(ContextState::new()).await.expect("converges");

    assert!(result.converged);
    assert!(result.context.has(ContextKey::Seeds));
}
```

Tokio is only the host runtime in this example. `converge-core` stays runtime-agnostic.

## Build Commands

If you're working on the Converge repo itself:

| Command | What it does |
|---|---|
| `just build` | `cargo build --release` |
| `just build-quick` | `cargo build --profile quick-release` |
| `just lint` | `cargo fmt --check && cargo clippy --all-targets -- -D warnings` |
| `just fix-lint` | Auto-fix lint issues |
| `just test` | `cargo test --all-targets` (default members) |
| `just test-all` | `cargo test --all-targets --workspace` |
| `just doc` | `cargo doc --no-deps --workspace` |
| `just example hello-convergence` | Run an example |

## Next Steps

1. Read [[Philosophy/Why Converge]] and [[Philosophy/Nine Axioms]] — understand the model before building
2. Read [[Building/Writing Agents]] — implement the `Suggestor` trait
3. Study [[Concepts/Context and Facts]] — understand the shared state model
4. Explore [[Concepts/Domain Packs]] — use pre-built suggestors for cross-cutting concerns

See also: [[Building/Crate Catalog]], [[Building/Context Keys]]
