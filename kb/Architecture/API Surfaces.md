---
tags: [architecture]
source: mixed
---
# API Surfaces

Converge exposes six external contracts. If a type, trait, or module is not
reachable through one of the surfaces below, it is an implementation detail and
must not be treated as a stable dependency.

Stabilization decisions live in [[Architecture/ADRs/ADR-001-canonical-public-crates]]
through [[Architecture/ADRs/ADR-007-capability-contract-ownership]].

Shared stack guidance for downstream consumers lives in [[Architecture/Golden Path Matrix]].

## Public Contracts

Converge exposes six external contracts:

1. Rust pack authoring
2. Rust provider capability routing
3. Rust semantic model
4. Rust in-process kernel embedding
5. Rust remote client
6. Remote wire protocol

These contracts are intentionally separate. A pack author should not need
provider selection APIs. A provider adapter should not need kernel internals. A
remote Rust consumer should not need CLI implementation code.

## Narrowest Surface Rule

Each layer should depend on the smallest public contract that satisfies its job.

- If code authors packs or invariants, use `converge-pack`.
- If code only needs chat contracts or routing vocabulary, use `converge-provider-api`.
- If code needs ready-made provider adapters, use `converge-provider` in controlled repos.
- If code embeds the Converge engine, use `converge-kernel`.
- If code only reads governed semantic outputs, use `converge-model`.

`converge-core` is not the default integration surface for next-layer consumers.
It is the engine and governance implementation crate.

## Supported Rust Crates

### `converge-pack`

Purpose:
- author pure Converge packs, suggestors, and invariants

What external code may use:
- `Suggestor`
- `Context`
- `ContextKey`
- `AgentEffect`
- `Fact`
- `ProposedFact`
- `Invariant`
- `InvariantClass`
- `InvariantResult`

Status:
- canonical pack authoring crate
- authoritative fact creation is kernel-gated
- pack authoring is proposal-only

Important note:
- `Suggestor` is the canonical authoring trait, and it cannot emit authoritative facts.
- `Suggestor::accepts()` is pure and synchronous; `Suggestor::execute()` is async and runtime-agnostic.
- `Fact` is read-only outside kernel-authority code paths.
- `AgentEffect` is proposal-only.

### `converge-provider-api`

Purpose:
- describe provider identity, capability contracts, chat request and response
  types, and routing requirements

What external code may use:
- `Backend`
- `BackendKind`
- `Capability`
- `ChatBackend`
- `DynChatBackend`
- `ChatRequest`
- `ChatResponse`
- `ChatMessage`
- `ChatRole`
- `ResponseFormat`
- `LlmError`
- `ToolDefinition`
- `ToolCall`
- `BackendError`
- `BackendErrorKind`
- `BackendRequirements`
- `BackendSelector`
- `SelectionCriteria`
- `Jurisdiction`
- `LatencyClass`
- `CostTier`
- `TaskComplexity`
- `RequiredCapabilities`
- `AgentRequirements`
- `ModelSelectorTrait`
- `ComplianceLevel`
- `CostClass`
- `DataSovereignty`

Status:
- canonical provider capability contract
- narrowest dependency for provider-capability consumers that do not need engine semantics

### `converge-model`

Purpose:
- expose the curated semantic model shared across kernel consumers

What external code may use:
- governed semantic types such as `Proposal`, `TypesFact`, `PromotionRecord`,
  `TypesRootIntent`, `Criterion`, `StopReason`, and related IDs/newtypes

Status:
- stable curated semantic surface
- authority-bearing fact construction is not part of this contract

### `converge-kernel`

Purpose:
- embed the Converge kernel in-process

What external code may use:
- `Engine`
- `ConvergeResult`
- `ConvergeError`
- `RunResult`
- `HitlPause`
- `EngineHitlPolicy`
- advanced HITL and flow-gate configuration types used by embedders
- `CriterionEvaluator`
- `ExperienceEventObserver`
- `TypesRunHooks`
- pack-facing traits re-exported for convenience

Status:
- canonical embedding surface
- current engine semantics enforce the single truth pipeline

### `converge-protocol`

Purpose:
- expose the generated Rust representation of the `converge.v1` wire contract

What external code may use:
- generated `v1::*` request, response, event, and service types
- `ConvergeService` client/server stubs generated from the protobuf contract
- re-exported `prost_types` for structured payloads

Status:
- canonical Rust representation of the remote wire contract
- semver must track the public `converge.v1` protocol

### `converge-client`

Purpose:
- connect to remote Converge runtimes from Rust without depending on CLI code

What external code may use:
- `ConvergeClient`
- `ClientError`
- `messages`
- `protocol` and `v1` re-exports for typed requests and events

Status:
- canonical remote Rust SDK
- intentionally thin over `converge-protocol`

## Deprecated Compatibility Surface

### `converge-traits`

Status:
- deprecated compatibility facade
- `publish = false`

Rule:
- no new code may depend on `converge-traits`
- controlled downstreams must migrate to `converge-pack` and
  `converge-provider-api`

## Who Uses What

| Consumer | Allowed Dependencies |
|---|---|
| Pack/module authors | `converge-pack`, `converge-model` |
| Embedded applications | `converge-kernel`, `converge-model`, `converge-pack` |
| Capability consumers | `converge-provider-api` |
| Provider adapters | `converge-provider-api` |
| Remote Rust consumers | `converge-client`, `converge-protocol` |
| Non-Rust consumers | `converge.v1` protobuf/gRPC |

## Remote Protocol

The current public network contract is the protobuf package in:

- `schema/proto/converge.proto`

This is the external client/server protocol for mobile, CLI, and remote
systems. The canonical Rust packaging of that contract is `converge-protocol`.

Status:
- public wire contract
- versioned by protobuf package (`converge.v1`)

The following is not the general external protocol:

- `schema/proto/kernel.proto`

`kernel.proto` is an internal service boundary for the GPU/kernel service and
must not be treated as the general external Converge API.

## Internal Crates

These crates are implementation and reference code, not stable external
contracts:

- `converge-core`
- `converge-provider`
- `converge-domain`
- `converge-runtime`
- `converge-application`
- `converge-storage`
- `converge-llm`
- `converge-optimization`
- `converge-analytics`
- `converge-mcp`
- `converge-knowledge`
- `converge-experience`
- `converge-axiom`
- `converge-remote`
- `converge-auth`
- `converge-consensus`
- `converge-ledger`
- `converge-nats`
- `converge-observability`

External code may use them experimentally inside controlled repos, but they are
not the promised public surface and may change without preserving compatibility.

## Downstream Mapping

| Project | Target API |
|---|---|
| organism | `converge-pack` + `converge-kernel` + `converge-model` + `converge-client` |
| axiom | `converge-provider-api` + `converge-provider` |
| helm (`saas-killer`) | `converge-kernel` + `converge-model` + `converge-pack` + `converge-provider-api`/`converge-provider` as needed |
| wolfgang | `converge-kernel` + `converge-model` + `converge-provider-api` + `converge-provider` |
| hackathon | `converge-pack` + `converge-kernel` + `converge-provider-api` + `converge-provider` |

## Contract Status

The breaking cut is complete:
- `Suggestor` is the public authoring trait (replaces `Agent`)
- `AgentEffect` is proposal-only (no `facts` field)
- `Fact` has no public constructor
- `SubmitObservationRequest` replaces `InjectFactRequest` in the wire protocol

## Regression Gate

Any PR touching pack/core/protocol/client/runtime must pass:
```bash
cargo test -p converge-pack --test compile_fail
cargo test -p converge-core --test compile_fail --test truth_pipeline --test negative --test properties
cargo test -p converge-client --test messages
```

## Semver Scope

Semver promises apply only to the six public crates and the `converge.v1` wire
protocol. Everything else is internal and may change without notice.

See also: [[Architecture/System Overview]], [[Architecture/Crate Map]], [[Architecture/Hexagonal Architecture]], [[Architecture/Known Drift]], [[Architecture/Golden Path Matrix]]
