# Converge

[![CI](https://github.com/Reflective-Lab/converge/actions/workflows/ci.yml/badge.svg)](https://github.com/Reflective-Lab/converge/actions/workflows/ci.yml)
[![Security](https://github.com/Reflective-Lab/converge/actions/workflows/security.yml/badge.svg)](https://github.com/Reflective-Lab/converge/actions/workflows/security.yml)
![coverage](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/kpernyer/0fa611de897a736cb134400aa56e05f3/raw/converge-coverage.json)
[![Crates.io](https://img.shields.io/crates/v/converge-pack.svg)](https://crates.io/crates/converge-pack)
[![docs.rs](https://docs.rs/converge-pack/badge.svg)](https://docs.rs/converge-pack)
[![dependency status](https://deps.rs/repo/github/Reflective-Lab/converge/status.svg)](https://deps.rs/repo/github/Reflective-Lab/converge)
![MSRV](https://img.shields.io/badge/MSRV-1.94.0-blue)
<img alt="gitleaks badge" src="https://img.shields.io/badge/protected%20by-gitleaks-blue">
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**A correctness-first, context-driven multi-agent runtime built in Rust.**

Agents collaborate through shared context, not by calling each other. The engine runs agents repeatedly until a fixed point is reached — convergence is explicit and observable. Every fact has provenance. Every decision has evidence. Every run either converges or stops honestly, reporting exactly why.

```
┌─────────────────────────────────────────────┐
│  Helm          Decision frameworks          │
├─────────────────────────────────────────────┤
│  Axiom         Truth validation & codegen   │
├─────────────────────────────────────────────┤
│  Organism      Reasoning, planning, debate  │
├─────────────────────────────────────────────┤
│  Converge      Engine, governance, commit   │  ← you are here
├─────────────────────────────────────────────┤
│  Providers     LLMs, tools, storage         │
└─────────────────────────────────────────────┘
```

## The Nine Axioms

These are inviolable. Every PR, every crate, every decision is evaluated against them.

1. **Explicit Authority** — No implicit permissions. Authority is typed, traceable, and enforced. Agents cannot promote their own proposals.
2. **Convergence Over Control Flow** — The engine runs agents in cycles until fixed point, not until a workflow completes.
3. **Append-Only Truth** — Facts are never mutated. Corrections are new facts. History is preserved.
4. **Agents Suggest, Engine Decides** — `ProposedFact` is not `Fact`. Every proposal passes through a promotion gate that validates authority, schema, and confidence.
5. **Safety by Construction** — Invalid states are unrepresentable. `unsafe_code = "forbid"` everywhere. Type-state enforcement on proposals → facts.
6. **Transparent Determinism** — All execution is deterministic and reproducible. Sequential core, serial effect merging in registration order.
7. **Human Authority is First-Class** — Human review is not a workaround. `CriterionResult::Blocked` with a typed approval reference is a valid, honest outcome.
8. **No Hidden Work** — All effects are visible in shared context. No shadow state, side channels, or implicit retries.
9. **Scale by Intent Replication** — Scaling never compromises governance. Every replica runs under the same rules.

## Quick Start

```bash
git clone https://github.com/Reflective-Lab/converge.git
cd converge

just build-quick    # build (fast iteration)
just test           # run tests
just lint           # format + clippy
```

### Your First Convergence

```rust
use async_trait::async_trait;
use converge_kernel::{
    AgentEffect, Context, ContextKey, ContextView, Engine, ProposedFact, Suggestor,
};

struct SeedSuggestor;

#[async_trait]
impl Suggestor for SeedSuggestor {
    fn name(&self) -> &str { "seed" }
    fn dependencies(&self) -> &[ContextKey] { &[] }
    fn accepts(&self, ctx: &dyn ContextView) -> bool {
        !ctx.has(ContextKey::Seeds)
    }
    async fn execute(&self, _ctx: &dyn ContextView) -> AgentEffect {
        AgentEffect::with_proposal(ProposedFact {
            key: ContextKey::Seeds,
            id: "observation-1".into(),
            content: "Monthly active users grew 15%".into(),
            confidence: 0.95,
            provenance: "suggestor:seed".into(),
        })
    }
}

#[tokio::main]
async fn main() {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor);
    let result = engine.run(Context::new()).await.expect("converges");
    assert!(result.converged);
}
```

## How Convergence Works

Think of a room of experts sharing a whiteboard. Each cycle, experts write proposals on sticky notes. A moderator validates and pins them. Experts react to the updated state. Eventually, no expert has anything new to add — fixed point reached.

```
initialize context from RootIntent
repeat
  determine eligible suggestors (pure, side-effect free)
  execute eligible suggestors (read-only context)
  collect AgentEffects (proposals only)
  promote proposals → facts (gate validates authority, schema, confidence)
  merge effects into context (serialized, deterministic)
  evaluate criteria
until convergence or termination
```

**Termination is always honest.** A run either:
- **Converges** — no new information changes the outcome, all acceptance invariants pass
- **Stops with a reason** — budget exhausted, invariant violated, or human review required

Every stop reason is typed. The system never silently gives up.

## The Promotion Gate

The most important design decision in Converge: **`ProposedFact` ≠ `Fact`**.

```
Observation → ProposedFact ──[promotion gate]──→ Fact (with PromotionRecord)
                                    │
                                    └──→ ValidationError (rejected)
```

Every promoted fact carries a complete audit trail: gate ID, policy version, approver, validation summary, evidence references, and trace link.

## Invariants

Executable guarantees — not validation functions, but definitions of what must always be true.

| Class | When Checked | On Violation |
|---|---|---|
| **Structural** | Every merge | Merge rejected immediately |
| **Semantic** | End of each cycle | Convergence blocked |
| **Acceptance** | When convergence claimed | Entire result rejected |

The system cannot claim success while violating its guarantees.

## Domain Packs

Pre-built suggestor collections for cross-cutting concerns. Register and run.

| Pack | Purpose |
|---|---|
| **trust** | Audit, access control, provenance, compliance, PII redaction |
| **money** | Invoicing, payment allocation, reconciliation, period close |
| **delivery** | Promise fulfillment, scope tracking, blockers, risk assessment |
| **knowledge** | Signal capture, hypothesis testing, canonical decisions |
| **data_metrics** | Metrics, dashboards, anomaly detection, alerting |

Each pack ships its own invariants. Custom suggestors and domain packs run under the same governance.

## Governed Flow Gates

Consequential transitions are policy-gated via Cedar:

- `converge-core` defines the neutral `FlowGateAuthorizer` contract
- `converge-policy` implements it with Amazon Cedar
- `converge-domain` applies it to real transitions: invoice issuance, period close, contract execution

Ed25519-signed delegation tokens with time-scoping and replay protection.

## Examples

```bash
just example hello-convergence   # core convergence loop
just example custom-agent        # implement the Suggestor trait
just example meeting-scheduler   # domain pack with constraints
just example custom-provider     # LLM provider adapter
just example vendor-selection    # multi-criteria selection + Cedar commit gating
just example expense-approval    # governed approval via FlowGateAuthorizer
just example loan-application    # HITL gate, domain packs
just example local-inference     # local LLM on Apple Silicon
just example gemma-inference     # Gemma GGUF via llama.cpp
```

See [examples/README.md](examples/README.md) for verified results and the full walkthrough.

## Canonical Public Crates

Six crates form the supported external API. Semver promises apply only to these.

| Crate | Role | For |
|---|---|---|
| [`converge-pack`](crates/pack) | Pack authoring (Suggestor, Invariant, Context) | Pack authors |
| [`converge-provider-api`](crates/provider-api) | Provider capability and routing contract | Provider adapters |
| [`converge-model`](crates/model) | Curated semantic model surface | Everyone |
| [`converge-kernel`](crates/kernel) | In-process embedding API | Embedded apps |
| [`converge-protocol`](crates/protocol) | Generated wire contract (`converge.v1`) | Remote systems |
| [`converge-client`](crates/client) | Remote Rust SDK | Remote Rust consumers |

```toml
# Embed the engine
[dependencies]
converge-kernel = "3"
converge-pack = "3"
converge-model = "3"
```

## Architecture

```
crates/
├── pack/           Canonical pack authoring contract
├── provider-api/   Canonical provider capability contract
├── model/          Curated semantic model surface
├── kernel/         In-process embedding API
├── protocol/       Generated wire contract (converge.v1)
├── client/         Remote Rust SDK
├── core/           Convergence engine implementation
├── provider/       LLM backends (14 providers)
├── domain/         Domain packs (trust, money, delivery, knowledge, metrics)
├── experience/     Event-sourced audit store
├── knowledge/      Vector knowledge base with GNN learning
├── mcp/            Model Context Protocol (client + server)
├── optimization/   Pure Rust solvers + optional OR-Tools FFI
├── storage/        Object storage (local, S3, GCS)
├── policy/         Cedar policy engine
├── auth/           Authentication, authorization, cryptography
├── consensus/      Raft consensus adapter
├── ledger/         Append-only context ledger
├── nats/           NATS messaging adapter
├── observability/  Audit, telemetry, metrics
├── llm/            Local LLM inference (Burn)
├── analytics/      ML/analytics pipeline (Polars + Burn)
├── runtime/        HTTP/gRPC execution service
└── application/    Reference distribution
```

## Capabilities

### LLM Providers

14 remote backends plus local inference:

Anthropic, OpenAI, Google Gemini, Ollama, Perplexity, Mistral, DeepSeek, Qwen, Grok (xAI), Kimi, Baidu ERNIE, Zhipu GLM, MinMax, OpenRouter — with fitness-based model selection, fallback chaining, and structured output (JSON, YAML, TOML, Markdown).

### Optimization

Pure Rust solvers — no external dependencies required:

Hungarian assignment, auction, push-relabel max flow, successive shortest paths, Dijkstra, 0-1 knapsack, set cover, list scheduling with disjunctive and cumulative resource constraints. Five domain solver packs with typed specs and determinism guarantees.

### Knowledge Base

Vector search with configurable HNSW, GNN learning layer, agentic memory (reflexion, skill library, causal reasoning, learning sessions, temporal), online learning with distribution drift detection, and meta-learning for few-shot strategy selection.

### Tool Integration

OpenAPI spec parsing, GraphQL introspection, MCP (Model Context Protocol) with stdio and HTTP transports, Brave Search, Tavily Search.

### Experience Store

Event-sourced audit ledger with three backends: in-memory (property-tested), SurrealDB (WebSocket, namespaced), LanceDB (vector-indexed, similarity search).

## Governance

24 personas across five categories — technical, business, legal, responsibility, and external perspectives — form a comprehensive review framework. Automated eval suites gate PR merges, release candidates, deployments, and marketing.

See [kb/Governance/](kb/Governance/) for the full framework.

## Guarantees

- `unsafe_code = "forbid"` across all crates
- Deterministic execution with seeded randomness
- Termination via cycle, fact, and token budgets
- Agent isolation — suggestors cannot affect each other
- Full provenance chain on every fact and proposal
- Supply chain auditing via `cargo-deny`

## Documentation

- [kb/Architecture/System Overview.md](kb/Architecture/System%20Overview.md) — system design and dependency graph
- [kb/Architecture/API Surfaces.md](kb/Architecture/API%20Surfaces.md) — canonical public contracts
- [kb/Philosophy/Nine Axioms.md](kb/Philosophy/Nine%20Axioms.md) — the inviolable design principles
- [kb/Philosophy/Convergence Explained.md](kb/Philosophy/Convergence%20Explained.md) — how fixed-point execution works
- [kb/Building/DEVELOPMENT.md](kb/Building/DEVELOPMENT.md) — setup, build, git workflow
- [kb/Governance/CONTRIBUTING.md](kb/Governance/CONTRIBUTING.md) — contribution guidelines
- [SECURITY.md](SECURITY.md) — vulnerability reporting and security baseline

## License

[MIT](LICENSE) — Copyright 2024–2026 Reflective Group AB
