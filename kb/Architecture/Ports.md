---
tags: [architecture]
source: mixed
---
# Ports

Ports are the trait definitions that form the boundary of the [[Architecture/Hexagonal Architecture|hexagon]]. The core defines what it needs. Adapters provide it.

> **Two layers, not one.** Ports come in two distinct levels: the
> *Suggestor* layer (purposeful, agency-aware) and the *Backend / Capability*
> layer (operational, capability-aware). A Suggestor uses Backends; the engine
> sees Suggestors. They must not be collapsed.
> See [[Architecture/Plug Boundary]] for the load-bearing rule.

## Authoring Ports (`converge-pack`)

These are the public contract. Every port is a trait. Every trait is `Send + Sync`.

### Suggestor Execution

| Port | Purpose | Key Methods |
|---|---|---|
| `Suggestor` | Capability contract | `name()`, `dependencies()`, `accepts()`, `execute().await` |
| `Context` | Read-only shared state | `has()`, `get()`, `get_proposals()`, `count()` |
| `Invariant` | Executable guarantees | `name()`, `class()`, `check()` |

`accepts()` is synchronous and pure. `execute()` is async but runtime-agnostic.

## Provider Routing Ports (`converge-provider`)

`converge-provider` is the provider contract crate. ADR-007 records the naming
rule: contracts get real domain names, while implementations carry
adapter-qualified names.

| Port | Purpose | Key Methods |
|---|---|---|
| `Backend` | Capability declaration | `name()`, `kind()`, `capabilities()`, `has_capability()` |
| `BackendSelector` | Capability-based routing | `select(&BackendRequirements)` |
| `ChatBackend` | Multi-turn LLM chat | `chat(ChatRequest)` |

### Backend Requirements

```rust
BackendRequirements::new(BackendKind::Llm)
    .with_capability(Capability::TextGeneration)
    .with_capability(Capability::Reasoning)
    .with_max_cost(CostClass::Medium)
    .with_data_sovereignty(DataSovereignty::EU)
    .with_compliance(ComplianceLevel::GDPR)
```

Selection is by capability, not by name ([[Concepts/Backends and Capabilities]]).

## Kernel Embedding Hooks (`converge-kernel`)

| Port | Purpose | Key Methods |
|---|---|---|
| `StreamingCallback` | Real-time notifications | `on_cycle_start()`, `on_fact()`, `on_cycle_end()` |
| `ExperienceEventObserver` | Experience event taps | `on_event(...)` |

## Internal Adapter Ports

| Port | Purpose | Key Methods |
|---|---|---|
| `DynChatBackend` | Runtime-polymorphic chat | `chat(ChatRequest)` |
| `EmbedBackend` | Vector embeddings | `embed(EmbedRequest)` |
| `WebSearchBackend` | Web search retrieval | `search_web(WebSearchRequest)` |
| `VectorRecall` | Similarity search | Vector-indexed retrieval |
| `Reranking` | Result re-ranking | Cross-encoder relevance scoring |

These are implementation details unless and until ADR-007 promotes them into a
provider or tool contract. In particular, web fetch, feed fetch, and search
DTOs need contract-owned types before external adapters can depend on them
without pulling in implementation crates.

## Error Contract

All ports use a consistent error type:

```rust
BackendError {
    kind: BackendErrorKind,
    message: String,
    retryable: bool,
}
```

Retryability is declared, not guessed: `RateLimit`, `Unavailable`, `Network`, `Timeout` are retryable. `Authentication`, `InvalidRequest` are not.

See also: [[Architecture/Plug Boundary]], [[Architecture/Providers]], [[Architecture/Hexagonal Architecture]]
