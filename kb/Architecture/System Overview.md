---
tags: [architecture]
source: mixed
---
# System Overview

Converge is a correctness-first, context-driven, multi-suggestor **library**.

It has no runtime. It does not own sockets, message buses, process lifecycle, or deployment. Those belong to [[Lattice]] — the execution mesh. Converge runs inside agent processes that participate in the Lattice mesh.

The kernel is pure. It owns the convergence loop, the promotion gate, runtime
invariants, typed stop reasons, HITL pauses, and the run integrity proof. It
does not own formation assembly, intent decomposition, outcome learning, app
orchestration, or transport.

For the full ecosystem picture see `~/dev/work/kb/Architecture/Ecosystem Overview.md`.

## System View (Converge internals)

```text
┌──────────────────────────────────────────┐
│ Consumer Layer                           │
│ Organism, Helms, apps (via kernel API)   │
└──────────────────────────────────────────┘
┌──────────────────────────────────────────┐
│ Converge Kernel                          │
│ Engine, promotion, invariants, integrity │
└──────────────────────────────────────────┘
┌──────────────────────────────────────────┐
│ Suggestor Layer                          │
│ Domain, policy, solver, analytics, LLM   │
│ (in-process or remote via Lattice/NATS)  │
└──────────────────────────────────────────┘
┌──────────────────────────────────────────┐
│ Provider / Tool Layer                    │
│ LLMs, storage, search, solvers, services │
└──────────────────────────────────────────┘
```

Suggestors that run out-of-process (e.g. Ferrox SAT) are reached through the
`Suggestor` trait — Lattice owns the transport (NATS). Converge never imports
Lattice.

## Workspace Layout

```text
crates/
  pack/           canonical pack authoring contract
  provider-api/   canonical provider capability contract
  model/          curated semantic model surface
  kernel/         canonical embedding API
  protocol/       generated converge.v1 wire contract
  client/         canonical remote Rust SDK
  core/           engine implementation and promotion path
  provider/       provider adapters
  domain/         built-in domain packs and governed flow logic
  policy/         Cedar policy engine and policy suggestors
  optimization/   solver packs and Suggestor adapter
  analytics/      analytics and ML suggestors
  knowledge/      knowledge base and knowledge suggestors
  experience/     experience event storage
  runtime/        HTTP and gRPC runtime
  storage/        object storage abstraction
```

## Dependency Shape

At a high level:

- `converge-pack`, `converge-provider-api`, and `converge-protocol` are the leaf contracts.
- `converge-core` builds on `converge-pack`.
- `converge-kernel` and `converge-model` sit above the engine implementation.
- Provider, domain, policy, optimization, analytics, and knowledge crates depend on contracts and the engine implementation as needed.

See [[Architecture/API Surfaces]] for the actual support boundary.

## Execution Model

```text
initialize context
repeat
  find eligible suggestors from dirty dependencies
  execute them against read-only context
  collect AgentEffects
  promote ProposedFacts into Facts
  merge promoted facts in deterministic registration order
  evaluate invariants and stop conditions
until convergence or typed termination
```

Key properties:

- context is the only shared state
- suggestors never call each other
- proposals become facts only through the engine
- deterministic ordering follows registration order
- semantic IDs, hashes, and timestamps cross the public contract as typed values
- wildcard matching at the runtime perimeter uses explicit selector types, not magic `"*"` strings
- runtime pack YAML is strict operational wiring: typed names and versions, explicit preset vocabulary, and no semantic-rule spillover
- budgets and typed stop reasons make termination explicit

## Built-In Packs vs Other Suggestors

`converge-domain` currently owns the built-in packs:

- `trust`
- `money`
- `delivery`
- `data_metrics`

Other internal crates still participate in the same loop through `Suggestor`:

- `converge-policy`
- `converge-optimization`
- `converge-analytics`
- `converge-knowledge`

There is no special side pipeline for them.

## Feature Notes

Notable feature-gated paths in the current workspace:

- `converge-pack/kernel-authority`
  `converge-core` enables this so the engine can construct authoritative `Fact` values.
- `converge-optimization/ffi`, `sat`, `full`
  `full` pulls in OR-Tools FFI and requires the local OR-Tools build tree.
- `converge-analytics/storage`, `excel`
  Optional storage and Excel ingest integrations.
- `converge-provider` backend features
  Enable concrete providers such as `openai`, `anthropic`, `gemini`, and others.

## Support Boundary

The supported external API consists of:

- `converge-pack`
- `converge-provider-api`
- `converge-model`
- `converge-kernel`
- `converge-protocol`
- `converge-client`

Everything else in this workspace is internal.

See also: [[Architecture/API Surfaces]], [[Architecture/Suggestor Contract]]
