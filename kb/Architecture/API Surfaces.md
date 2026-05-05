---
tags: [architecture]
source: mixed
---
# API Surfaces

Converge exposes six external contracts. If a type or module is not reachable
through one of these surfaces, it is an implementation detail and should not be
treated as a stable dependency.

## Narrowest Surface Rule

Depend on the smallest public contract that satisfies the job:

- authoring packs or invariants: `converge-pack`
- routing providers or chat backends: `converge-provider-api`
- reading governed semantic outputs: `converge-model`
- embedding the engine: `converge-kernel`
- calling a remote runtime from Rust: `converge-client`
- integrating over the wire: `converge-protocol`

`converge-core` is not the default downstream integration surface.

## Mantra

For public API placement, keep repeating the rule until it becomes muscle memory:

- semantics in `converge-model`
- authoring in `converge-pack`
- runnable machinery in `converge-kernel`

## Public Contracts

### `converge-pack`

Purpose:
- author suggestors, invariants, and proposal-only effects

Key surface:
- `Suggestor`
- `Context`
- `ContextKey`
- `AgentEffect`
- `ProposedFact`
- `Fact` (read-only outside kernel-authority code)
- semantic contract values such as `FactId`, `ProposalId`, `GateId`, `Timestamp`, and `ContentHash`
- `Invariant`

Status:
- canonical authoring contract
- authoritative fact creation is feature-gated
- semantic identifiers, hashes, and timestamps are typed contract values, not string conventions

### `converge-provider-api`

Purpose:
- backend identity, capability routing, chat contracts, and selection criteria
- provider selection request / assignment payloads

Key surface:
- `Backend`
- `Capability`
- `ChatBackend`
- `ChatRequest`
- `ChatResponse`
- `ChatMessage`
- `ResponseFormat`
- `BackendSelector`
- `SelectionCriteria`

Status:
- canonical provider capability contract

### `converge-model`

Purpose:
- curated semantic types shared across kernel consumers
- formation semantics shared across embedders and upper layers

Key surface:
- semantic output types
- IDs and newtypes
- intent and criterion vocabulary intended for consumers

Status:
- curated semantic surface for downstream consumers

### `converge-kernel`

Purpose:
- embed the engine in-process
- expose grouped batteries-included formation machinery for embedders

Key surface:
- `Engine`
- `ContextState`
- `ConvergeResult`
- `ConvergeError`
- `RunResult`
- `HitlPause`
- `CriterionEvaluator`
- `ExperienceEventObserver`
- re-exported pack-facing traits for embedders

Status:
- canonical in-process embedding surface

## Formation Pattern Across Public Crates

Formation support is intentionally split across the canonical surfaces:

- `converge-model`
  - `FormationRequest`
  - `FormationPlan`
  - `ProfileSnapshot`
  - `SuggestorRole`
  - `SuggestorCapability`
  - `SuggestorProfile`
- `converge-provider-api`
  - `ProviderRequest`
  - `ProviderAssignment`
  - `CapabilityAssignment`
- `converge-kernel::formation`
  - grouped re-exports of the semantic types above
  - `FormationAssemblySuggestor`
  - `ProviderSelectionSuggestor`

That split is deliberate: structure lives in the semantic crates, but runnable
machinery is easy to find from the embedding surface.

The public split is enforced with consumer-style compile-pass tests in
`converge-model`, `converge-provider-api`, and `converge-kernel`.

### `converge-protocol`

Purpose:
- generated Rust packaging of the `converge.v1` wire contract

Key surface:
- generated `v1::*` messages and service types

Status:
- canonical Rust representation of the public wire protocol

### `converge-client`

Purpose:
- typed Rust SDK for remote Converge runtimes

Key surface:
- `ConvergeClient`
- `ClientError`
- `messages`
- `protocol` / `v1` re-exports

Status:
- canonical remote Rust SDK

## Who Uses What

| Consumer | Preferred dependencies |
|---|---|
| Pack authors | `converge-pack`, optionally `converge-model` |
| Embedded applications | `converge-kernel`, optionally `converge-model` |
| Provider adapters | `converge-provider-api` |
| Remote Rust consumers | `converge-client`, optionally `converge-protocol` |
| Non-Rust consumers | protobuf/gRPC `converge.v1` |

## Internal Crates

These workspace crates are real and useful, but they are not canonical public
contracts:

- `converge-core`
- `converge-provider`
- `converge-domain`
- `converge-policy`
- `converge-optimization`
- `converge-analytics`
- `converge-knowledge`
- `converge-experience`
- `converge-runtime`
- `converge-storage`

Some of these crates are publishable. That does not make them part of the
stable API promise.

## Remote Protocol

The current network contract is the protobuf package in:

- `schema/proto/converge.proto`

That contract is surfaced to Rust through `converge-protocol`.

See also: [[Architecture/Crate Map]], [[Architecture/System Overview]]
