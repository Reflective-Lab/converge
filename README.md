# Converge.zone

**Converge** is a correctness-first, context-driven multi-agent runtime built in Rust.

Agents collaborate through shared context, not by calling each other. The engine runs agents repeatedly until a fixed point is reached — convergence is explicit and observable.

## Design Principles

1. **Agents suggest, engines decide.** `ProposedFact` is not `Fact`. Every proposal carries confidence and provenance. The promotion gate validates before promoting.
2. **Context is the API.** Agents communicate through shared context, never by calling each other.
3. **Append-only truth.** Facts are never mutated; corrections are new facts.
4. **Convergence is observable.** `CriterionEvaluator` checks success conditions. `CriterionResult::Blocked` triggers honest stopping with `HumanInterventionRequired`.
5. **Truths are typed.** `TruthDefinition` declares jobs, policies, and invariants with success criteria, constraints, and pack participation — not string descriptions.
6. **Consequential transitions are policy-gated.** Real business flow actions project into a neutral `FlowGateAuthorizer` contract, with Cedar as the default implementation.
7. **Safety by construction.** `unsafe_code = "forbid"` everywhere. Type-state enforcement on proposals → facts.

## Quick Start

```bash
git clone https://github.com/Reflective-Lab/converge.git
cd converge

just build-quick    # build (fast iteration)
just test           # run tests
just lint           # format + clippy
```

## Examples

```bash
just example hello-convergence   # engine loop, agents, facts
just example custom-agent        # implement the Suggestor trait
just example meeting-scheduler   # domain pack with constraints
just example custom-provider     # implement an LLM provider
```

See [examples/README.md](examples/README.md) for the full list.

## Governed Flow Gates

Converge now has a default gate path for consequential flow transitions:

- `converge-core` defines the neutral `FlowGateAuthorizer` contract
- `converge-policy` implements that contract with Cedar
- `converge-domain` applies it to real pack transitions like invoice issuance, period close, and contract execution

The same shape is exercised in the governed examples:

- `expense-approval` — finance commit escalation and approval
- `vendor-selection` — procurement commit escalation and approval

## Architecture

```
crates/
├── pack/          # Canonical pack authoring contract
├── provider-api/  # Canonical provider capability contract
├── model/         # Curated semantic model surface
├── kernel/        # Curated in-process embedding API
├── protocol/      # Generated wire contract (converge.v1)
├── client/        # Canonical remote Rust SDK
├── core/          # Convergence engine (implementation)
├── traits/        # Deprecated compatibility facade
├── provider/      # LLM backends (Anthropic, OpenAI, Gemini, Ollama, ...)
├── domain/        # Domain packs (scheduling, routing, drafting, governed flows)
├── experience/    # Event-sourced audit store
├── knowledge/     # Vector knowledge base
├── mcp/           # Model Context Protocol (client + server)
├── optimization/  # Constraint solving (OR-Tools)
├── storage/       # Object storage abstraction
├── policy/        # Cedar policy engine and default flow gate authorizer
├── auth/          # Authentication, authorization, and cryptography
├── consensus/     # Raft consensus adapter for distributed Converge
├── ledger/        # Append-only context ledger
├── nats/          # NATS messaging adapter
├── observability/ # Audit, telemetry, and metrics
├── llm/           # Local LLM inference (Burn)
├── analytics/     # ML/analytics agents
├── tool/          # Development toolchain (Gherkin, JTBD)
├── runtime/       # HTTP/gRPC execution service
├── remote/        # Compatibility CLI on top of client + protocol
└── application/   # Reference distribution
examples/
├── hello-convergence/   # Minimal convergence loop
├── custom-agent/        # Implement the Suggestor trait
├── meeting-scheduler/   # Domain pack with constraints
├── custom-provider/     # LLM provider adapter
├── vendor-selection/    # Multi-criteria vendor selection with Cedar commit gating
├── expense-approval/    # Governed expense approval via default flow gate contract
├── loan-application/    # Loan application processing
├── local-inference/     # Local inference on Apple Silicon
└── gemma-inference/     # Gemma GGUF inference via llama.cpp
```

## Canonical Public Crates

Six crates form the supported external API (see [ADR-001](kb/Architecture/ADRs/ADR-001-canonical-public-crates.md)):

| Crate | Role |
|-------|------|
| `converge-pack` | Pack authoring contract (Suggestor, Invariant, Context) |
| `converge-provider-api` | Provider capability and routing contract |
| `converge-model` | Curated semantic model surface |
| `converge-kernel` | In-process embedding API |
| `converge-protocol` | Generated wire contract (`converge.v1`) |
| `converge-client` | Remote Rust SDK |

## Configuration

```env
CONVERGE_LLM_BACKEND=ndarray
CONVERGE_LLM_MODEL=llama3
CONVERGE_STORAGE_BACKEND=lancedb
CONVERGE_STORAGE_PATH=./data
RUST_LOG=info
```

## Documentation

- [kb/Architecture/System Overview.md](kb/Architecture/System%20Overview.md) — system design and dependency graph
- [kb/Architecture/API Surfaces.md](kb/Architecture/API%20Surfaces.md) — canonical public contracts
- [DEVELOPMENT.md](DEVELOPMENT.md) — setup, build, git workflow (worktrees, jj)
- [CONTRIBUTING.md](CONTRIBUTING.md) — contribution guidelines
- [SECURITY.md](SECURITY.md) — vulnerability reporting and security baseline
- [crates/provider/.env.example](crates/provider/.env.example) — provider API key and endpoint catalog
- [examples/](examples/) — runnable examples

## License

MIT — see [LICENSE](LICENSE).

Copyright 2024–2026 Reflective Group AB

Kenneth Pernyer — [kenneth@reflective.se](mailto:kenneth@reflective.se)

Project: [github.com/Reflective-Lab/converge](https://github.com/Reflective-Lab/converge)
