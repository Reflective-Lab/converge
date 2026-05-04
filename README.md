# Converge

[![CI](https://github.com/Reflective-Lab/converge/actions/workflows/ci.yml/badge.svg)](https://github.com/Reflective-Lab/converge/actions/workflows/ci.yml)
[![Security](https://github.com/Reflective-Lab/converge/actions/workflows/security.yml/badge.svg)](https://github.com/Reflective-Lab/converge/actions/workflows/security.yml)
![coverage](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/kpernyer/0fa611de897a736cb134400aa56e05f3/raw/converge-coverage.json)
[![Crates.io](https://img.shields.io/crates/v/converge-pack.svg)](https://crates.io/crates/converge-pack)
[![docs.rs](https://docs.rs/converge-pack/badge.svg)](https://docs.rs/converge-pack)
[![dependency status](https://deps.rs/repo/github/Reflective-Lab/converge/status.svg)](https://deps.rs/repo/github/Reflective-Lab/converge/status.svg)
![MSRV](https://img.shields.io/badge/MSRV-1.94.0-blue)
<img alt="gitleaks badge" src="https://img.shields.io/badge/protected%20by-gitleaks-blue">
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**A correctness-first, context-driven multi-suggestor runtime built in Rust.**

Converge owns the convergence loop, the promotion gate, runtime invariants, HITL pauses, and the integrity proof of a run. It does **not** own formation assembly, intent decomposition, outcome learning, or application orchestration. Those belong above the kernel in systems such as Organism and Helms.

Everything that participates in the loop does so through one trait: `Suggestor`.

```
┌─────────────────────────────────────────────┐
│  Helms         App orchestration            │
├─────────────────────────────────────────────┤
│  Organism      Formation assembly           │
├─────────────────────────────────────────────┤
│  Converge      Engine, promotion, integrity │  ← you are here
├─────────────────────────────────────────────┤
│  Providers     LLMs, tools, storage         │
└─────────────────────────────────────────────┘
```

## What Converge Guarantees

- Suggestors propose. The engine promotes.
- `ProposedFact` is freely constructible; authoritative `Fact` construction is kernel-gated.
- The core loop is deterministic: eligibility and merge order follow registration order, not ad hoc side channels.
- Every promoted fact carries provenance and participates in the run integrity proof.
- Human review is a first-class stop condition, not an exception path.

## Quick Start

```bash
git clone https://github.com/Reflective-Lab/converge.git
cd converge

just build-quick
just test
just lint
```

## Your First Convergence

```rust
use converge_kernel::{
    AgentEffect, Context, ContextKey, ContextState, Engine, ProposedFact, Suggestor,
};

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

    async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
        AgentEffect::with_proposal(
            ProposedFact::new(
                ContextKey::Seeds,
                "observation-1",
                "Monthly active users grew 15%",
                "suggestor:seed",
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

`converge_kernel::Context` is the trait suggestors read from. `ContextState` is the concrete state embedders create.

## One Universal Contract

Every loop participant is a `Suggestor`. There are no side-car pipeline traits for policy, optimization, analytics, or knowledge.

| Kind | Typical implementation | Registration path |
|---|---|---|
| Built-in domain logic | `converge-domain` agents and packs | `register_suggestor()` or `register_suggestor_in_pack()` |
| Policy and governance | `PolicyGateSuggestor`, `FlowGateSuggestor` | `register_suggestor()` |
| Optimization | `SolverSuggestor<P>` | `register_suggestor()` |
| Analytics / ML | `FeatureAgent`, training and inference suggestors | `register_suggestor()` |
| Knowledge | `KnowledgeRetrievalSuggestor`, `KnowledgeStoreSuggestor` | `register_suggestor()` |
| Custom LLMs and tools | any crate implementing `Suggestor` | `register_suggestor()` |

The mixed formation example shows this explicitly: one engine run combining an intent seeder, an optimization solver, a policy gate, and an LLM-style evaluator. See [examples/formation-mixed](examples/formation-mixed/).

## Formation Offering API

The stable formation pattern follows one rule:

- semantics in `converge-model`
- authoring in `converge-pack`
- runnable machinery in `converge-kernel`

For embedders, the grouped entrypoint is `converge_kernel::formation`:

- semantic formation types such as `FormationRequest`, `FormationPlan`, `ProfileSnapshot`, `SuggestorRole`, and `SuggestorCapability`
- provider selection payloads such as `ProviderRequest` and `ProviderAssignment`
- built-in machinery such as `FormationAssemblySuggestor` and `ProviderSelectionSuggestor`

The canonical structured boundary begins at `FormationRequest` and `ProviderRequest`.

- If intent already arrives structured, a seeder can write those requests directly.
- If intent is loose, an upstream suggestor such as an intent codec should compile it into those requests first.

Both patterns are valid. The formation contract starts at the structured requests, not at the upstream compiler.

## Canonical Public Crates

Six crates define the supported external API. Semver promises apply to these surfaces.

| Crate | Purpose |
|---|---|
| [`converge-pack`](crates/pack) | Author packs, suggestors, invariants |
| [`converge-provider-api`](crates/provider-api) | Backend identity, capability routing, and provider selection payloads |
| [`converge-model`](crates/model) | Curated semantic types, including formation semantics |
| [`converge-kernel`](crates/kernel) | In-process embedding API and grouped formation machinery |
| [`converge-protocol`](crates/protocol) | Generated `converge.v1` wire types |
| [`converge-client`](crates/client) | Remote Rust SDK |

Everything else in the workspace is internal implementation or internal batteries-included functionality. Some of those crates are publishable today; that does **not** make them canonical public contracts.

## Built-In Packs and Internal Suggestor Crates

`converge-domain` currently ships the built-in domain packs:

- `trust`
- `money`
- `delivery`
- `data_metrics`

Other internal crates join the **same** convergence loop through `Suggestor` adapters instead of separate pipeline machinery:

- `converge-policy`
- `converge-optimization`
- `converge-analytics`
- `converge-knowledge`

That split is deliberate. Domain packs are not a dumping ground for every subsystem.

## Examples

The workspace currently ships these example crates:

- `hello-convergence`
- `custom-agent`
- `custom-provider`
- `meeting-scheduler`
- `expense-approval`
- `vendor-selection`
- `loan-application`
- `formation-mixed`
- `intent-codec-loop`
- `live-formation`
- `adaptive-gap-loop`
- `fixed-point-vs-budget`
- `reconciliation-loop`
- `analytics-packs`

Business-shaped examples such as `expense-approval`, `vendor-selection`, and
`loan-application` are Converge fixtures for kernel behavior: convergence,
policy gates, HITL pauses, and proposal promotion. They are not the canonical
organizational workflow implementations. Reusable approval, procurement, vendor,
and planning logic belongs downstream in Organism/domain packs or application
layers that consume Converge.

```bash
just examples
just example hello-convergence
just example adaptive-gap-loop
just example fixed-point-vs-budget
just example reconciliation-loop
just example formation-mixed
just example intent-codec-loop
just example live-formation
```

`cargo build --workspace` compiles the examples as part of the workspace build.

If you want the genuinely adaptive loop story rather than a bounded demo, start
with `adaptive-gap-loop`. It shows suggestors discovering new work, reopening
the loop, and converging only when the discovered graph is actually closed.

If you want the stop-semantics story, continue with `fixed-point-vs-budget`.
It runs the same adaptive graph loop twice and shows the real kernel contract:
fixed point returns `ConvergeResult`, while budget stop returns
`ConvergeError::BudgetExhausted`. It also uses real Dijkstra frontier planning
from `converge-optimization`.

If you want a larger data-massaging example that still belongs in Converge,
continue with `reconciliation-loop`. It takes two noisy ledgers, scores pair
quality, and uses exact Hungarian assignment from `converge-optimization` to
produce a one-to-one reconciliation plus explicit residue.

If you want the formation story rather than a single pre-arranged loop, start
with `intent-codec-loop`. It shows a loose Gherkin-ish DD spec compiled into a
`FormationRequest` and `ProviderRequest`, then answered by
`FormationAssemblySuggestor` and `ProviderSelectionSuggestor` inside one real
engine run.

If you want the fully self-assembling story, continue with `live-formation`.
It uses the same contract, but starts from direct structured requests instead
of an intent codec and lets the engine assemble a five-member loop from a
catalog plus backend pool.

## Workspace Commands

```bash
just build             # cargo build --release
just build-quick       # cargo build --profile quick-release
just check             # cargo check --workspace
just test              # cargo test --all-targets
just test-all          # cargo test --all-targets --workspace
just test-bench        # compile benchmarks
just test-soak         # long-running soak tests
just sec-gate          # security regression gate (policy + runtime + compile-fail)
just sec-deny          # dependency audit via cargo-deny
just lint              # cargo fmt --check && cargo clippy --all-targets -- -D warnings
just doc               # cargo doc --no-deps --workspace
just focus             # session opener — repo state + workspace health
just sync              # status + recent commits
just status            # recent test tail + commit summary
just git-hygiene       # worktrees, branches, release tag, cleanup candidates
just size-audit        # converge-runtime / converge-kernel packaging baseline
```

## Workspace Layout

```text
crates/
  pack/             Canonical pack authoring contract
  provider-api/     Canonical provider capability contract
  model/            Curated semantic model surface
  kernel/           Canonical in-process embedding API
  protocol/         Canonical generated wire contract
  client/           Canonical remote Rust SDK
  core/             Engine implementation and promotion gate
  provider/         Provider adapters
  domain/           Built-in domain packs and governed flow logic
  policy/           Cedar policy engine and policy suggestors
  optimization/     Solver packs and Suggestor adapter
  analytics/        Analytics and ML suggestors
  knowledge/        Knowledge base and knowledge suggestors
  experience/       Experience event storage
  runtime/          HTTP and gRPC runtime
  storage/          Object storage abstraction
```

## Documentation

The knowledge base in `kb/` is the canonical project documentation.

- [API Surfaces](kb/Architecture/API%20Surfaces.md)
- [Embedding Quick Start](kb/Architecture/Embedding%20Quick%20Start.md)
- [Formation Pattern](kb/Architecture/Formation%20Pattern.md)
- [Suggestor Contract](kb/Architecture/Suggestor%20Contract.md)
- [Crate Catalog](kb/Building/Crate%20Catalog.md)
- [Examples Guide](examples/README.md)
