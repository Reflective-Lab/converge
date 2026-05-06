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

## A New World

For decades software ran on a hard constraint: machines need explicit, unambiguous instructions. Every layer above the metal — languages, frameworks, runtimes — existed to remove ambiguity from human intent *before* execution.

That floor has shifted. Models and orchestration can now interpret intent, generate candidate actions, evaluate them against constraints, and iterate at runtime. The opportunity is not to remove structure but to **relocate** it — out of hardcoded instructions and into constraints, guardrails, contracts, promotion gates, and evaluation loops.

Converge and the platform around it provide that relocated structure:

- **Converge** — the convergence kernel: promotion gates, invariants, HITL pauses, integrity proof. The *whether*.
- **Organism** — formation assembly, intent decomposition, debate. The *how*.
- **Axiom** — truth definitions and projections. The *what is true*.
- **Helms** — operator control surfaces. The *what is shown*.

Together they let intent become a governed decision *at runtime*, safely — with provenance, authority, and an honest stop reason on every run. A new world built on adaptive ceilings collapses without a deterministic floor; Converge is that floor. Convergence-as-fixed-point, kernel-gated `Fact` construction, and HITL at irreversibles are how this stack stays trustworthy when the layers above are intent-driven and probabilistic.

The bigger picture is the shift from *tools that capture inputs* to *systems that produce defensible outcomes* — see [kb/Philosophy/Why We Build](kb/Philosophy/Why%20We%20Build.md) for the full thesis, including how this reshapes SaaS and the way organizations need to rethink architecture and ownership.

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

| Kind | Typical implementation | Home |
|---|---|---|
| Optimization | `SolverSuggestor<P>` | `crates/optimization` (foundation) |
| Domain packs | trust, money, delivery, data_metrics agents | [`atelier`](https://github.com/Reflective-Lab/atelier) showcase repo |
| Policy and governance | `PolicyGateSuggestor`, `FlowGateSuggestor` | [`arbiter`](https://github.com/Reflective-Lab/arbiter) extension |
| Analytics / ML | `FeatureAgent`, training and inference suggestors | [`prism`](https://github.com/Reflective-Lab/prism) extension |
| Knowledge | retrieval and store suggestors | [`mnemos`](https://github.com/Reflective-Lab/mnemos) extension |
| Generic providers and tools | LLM, search, fetch, feed, embedding, OpenAPI/GraphQL adapters | [`manifold-adapters`](https://github.com/Reflective-Lab/manifold-adapters) extension |
| Source-specific connectors | LinkedIn, Stripe, OCR, ... | [`embassy`](https://github.com/Reflective-Lab/embassy) extension |
| Native solvers | OR-Tools, CP-SAT, ... | [`ferrox`](https://github.com/Reflective-Lab/ferrox) extension |
| Custom LLMs and tools | any crate implementing `Suggestor` | your crate |

All of them register through the same call: `engine.register_suggestor(...)`. See [kb/Architecture/Extension Topology](kb/Architecture/Extension%20Topology.md) for the full extension map and [kb/Architecture/Core Ideas](kb/Architecture/Core%20Ideas.md) for the durable principles.

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
| [`converge-provider`](crates/provider) | Backend identity, capability routing, and provider selection payloads |
| [`converge-model`](crates/model) | Curated semantic types, including formation semantics |
| [`converge-kernel`](crates/kernel) | In-process embedding API and grouped formation machinery |
| [`converge-protocol`](crates/protocol) | Generated `converge.v1` wire types |
| [`converge-client`](crates/client) | Remote Rust SDK |

Everything else in the workspace is internal implementation or internal batteries-included functionality. Some of those crates are publishable today; that does **not** make them canonical public contracts.

## Foundation vs Extensions

Foundation owns universal contracts and the convergence kernel. Implementation-heavy capabilities — vector stores, ML pipelines, policy engines, source-specific connectors, native solvers, vendor SDKs — live in extension repositories under `~/dev/extensions/*` and consume foundation contracts through `Suggestor` adapters.

The dependency arrow is one-way: **foundation contracts ← extensions ← products**. Foundation never imports an extension.

Provider/tool dependencies follow the same rule:

```text
converge-provider  <-  manifold  <-  products / runway
```

`converge-provider` is the API. Adapter implementations use qualified names
and live in Manifold. The former `crates/provider-adapters` staging crate has
been removed; generic LLM, search, fetch, feed, embedding/reranking, vector,
and OpenAPI/GraphQL tool adapters now live in Manifold.
Host assembly supplies already-constructed handles through
`ChatBackendRegistry`; Converge selects handles, it does not instantiate vendor
SDKs.

| Repo | Contains | Extracted |
|---|---|---|
| [`atelier`](https://github.com/Reflective-Lab/atelier) | Showcase: domain packs (trust, money, delivery, data_metrics) plus worked examples | 2026-05-05 |
| [`mnemos`](https://github.com/Reflective-Lab/mnemos) | Knowledge retrieval and storage suggestors | 2026-05-05 |
| [`prism`](https://github.com/Reflective-Lab/prism) | Analytics and ML suggestors (feature, training, inference, monitoring) | 2026-05-05 |
| [`arbiter`](https://github.com/Reflective-Lab/arbiter) | Cedar policy engine and policy suggestors | 2026-05-05 |
| [`manifold-adapters`](https://github.com/Reflective-Lab/manifold-adapters) | Generic provider/tool adapters: LLM, search, fetch, feed, embeddings, external tools | active |
| [`embassy`](https://github.com/Reflective-Lab/embassy) | Source-specific connector ports (LinkedIn, ...) | 2026-05-05 |
| [`ferrox`](https://github.com/Reflective-Lab/ferrox) | Native solver bridge (OR-Tools / CP-SAT) | relocated 2026-05-05 |

See [kb/Architecture/Extension Topology](kb/Architecture/Extension%20Topology.md) for the full map.

## Examples

Worked examples live in the [`atelier`](https://github.com/Reflective-Lab/atelier) showcase repo (extracted from this workspace on 2026-05-05). It contains the canonical hello-convergence, custom-agent, custom-provider, meeting-scheduler, expense-approval, vendor-selection, loan-application, formation-mixed, intent-codec-loop, live-formation, adaptive-gap-loop, fixed-point-vs-budget, reconciliation-loop, and analytics-packs examples.

Business-shaped examples such as `expense-approval`, `vendor-selection`, and `loan-application` are kernel-behavior fixtures (convergence, policy gates, HITL pauses, proposal promotion). They are not canonical organizational workflow implementations. Reusable approval, procurement, vendor, and planning logic belongs downstream in Organism/domain packs or applications that consume Converge.

Recommended reading order, in atelier:

- `adaptive-gap-loop` — suggestors discovering new work, reopening the loop, converging only when the discovered graph is closed
- `fixed-point-vs-budget` — the real stop semantics: fixed point returns `ConvergeResult`, budget stop returns `ConvergeError::BudgetExhausted`
- `reconciliation-loop` — exact Hungarian assignment from `converge-optimization` over two noisy ledgers
- `intent-codec-loop` — loose Gherkin-ish DD spec compiled into `FormationRequest` and `ProviderRequest`, answered inside one engine run
- `live-formation` — five-member loop assembled from a catalog plus backend pool, starting from direct structured requests

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
  provider/         Canonical provider capability contract
  model/            Curated semantic model surface
  kernel/           Canonical in-process embedding API
  protocol/         Canonical generated wire contract
  client/           Canonical remote Rust SDK
  core/             Engine implementation and promotion gate
  optimization/     Solver packs and Suggestor adapter
  experience/       Experience event storage
  runtime/          HTTP and gRPC runtime
  storage/          Object storage abstraction
```

Domain packs, policy, analytics, and knowledge moved to extension repos on 2026-05-05 — see [Foundation vs Extensions](#foundation-vs-extensions).

## Documentation

The knowledge base in `kb/` is the canonical project documentation.

- [Developer Guide](kb/Building/Developer%20Guide.md) — end-to-end guide for Converge 3.8.1 developers
- [Core Ideas](kb/Architecture/Core%20Ideas.md) — durable principles for the next stable period
- [API Surfaces](kb/Architecture/API%20Surfaces.md)
- [Extension Topology](kb/Architecture/Extension%20Topology.md) — where extension code lives and why
- [Embedding Quick Start](kb/Architecture/Embedding%20Quick%20Start.md)
- [Formation Pattern](kb/Architecture/Formation%20Pattern.md)
- [Suggestor Contract](kb/Architecture/Suggestor%20Contract.md)
- [Crate Catalog](kb/Building/Crate%20Catalog.md)
