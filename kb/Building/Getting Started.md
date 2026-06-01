---
tags: [building]
source: mixed
---
# Getting Started

## Add Converge to Your Project

Start with the kernel:

```toml
[dependencies]
converge-kernel = "3"
async-trait = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Add narrower crates only when you need them:

```toml
converge-pack = "3"      # author reusable packs and invariants directly
converge-model = "3"     # curated semantic types
converge-client = "3"    # remote Rust client
```

Built-in domain packs and internal suggestor crates exist in this workspace,
but they are not part of the six canonical public API surfaces. See
[[Building/Crate Catalog]].

## First Run

```rust
use converge_kernel::{
    AgentEffect, Context, ContextKey, ContextState, Engine, Provenance, ProvenanceSource,
    Suggestor, TextPayload,
};

#[derive(Clone, Copy, Debug)]
struct SeedProvenance;

impl ProvenanceSource for SeedProvenance {
    fn as_str(&self) -> &'static str {
        "suggestor:seed"
    }
}

const SEED_PROVENANCE: SeedProvenance = SeedProvenance;

struct SeedSuggestor;

#[async_trait::async_trait]
impl Suggestor for SeedSuggestor {
    fn name(&self) -> &str {
        "seed"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        !ctx.has(ContextKey::Seeds)
    }

    fn provenance(&self) -> Provenance {
        SEED_PROVENANCE.provenance()
    }

    async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
        AgentEffect::with_proposal(
            SEED_PROVENANCE.proposed_fact(
                ContextKey::Seeds,
                "observation-1",
                TextPayload::new("Monthly active users grew 15%"),
            )
            .with_confidence(0.95),
        )
    }
}

#[tokio::main]
async fn main() {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor);

    let result = engine
        .run(ContextState::new())
        .await
        .expect("should converge");

    assert!(result.converged);
    assert!(result.context.has(ContextKey::Seeds));
}
```

Tokio is only the host runtime in this example. `converge-core` stays
runtime-agnostic.

## Build Commands

If you're working inside the Converge repo itself:

| Command | What it does |
|---|---|
| `just build` | `cargo build --release` |
| `just build-quick` | `cargo build --profile quick-release` |
| `just check` | `cargo check --workspace` |
| `just lint` | `cargo fmt --check && cargo clippy --all-targets -- -D warnings` |
| `just test` | `cargo test --all-targets` |
| `just test-all` | `cargo test --all-targets --workspace` |
| `just doc` | `cargo doc --no-deps --workspace` |
| `just example formation-mixed` | Run the mixed Suggestor loop example |

## Next Steps

1. Read [[Philosophy/Nine Axioms]].
2. Read [[Architecture/Embedding Quick Start]].
3. Read [[Architecture/Suggestor Contract]].
4. Read [[Concepts/Domain Packs]] to see where built-in packs stop and other suggestor families begin.

See also: [[Building/Crate Catalog]], [[Building/Context Keys]]
