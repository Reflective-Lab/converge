---
tags: [architecture, philosophy, load-bearing]
source: mixed
date: 2026-05-05
---
# Plug Boundary — Two Layers of Pluggable Interfaces

> **THE TRUTH:** Converge has two pluggable interfaces, not one. They live at
> different levels and must not be collapsed.

Conflating them is the trap behind the `kernel-authority` feature drift, the
`provider-api` naming inversion, and the temptation to let a Suggestor import
an adapter type. Keep them separate.

## The Two Layers

| Layer | What | Knows | Plug Type |
|---|---|---|---|
| **Suggestor** | Purposeful contributor to convergence. Reads `ContextFact`, emits `ProposedFact` and finalized `AgentEffect`. | What is being built and why. Has agency. | High-level. The unit of "a thing in a Formation." |
| **Backend / Capability** | Operational adapter. Declares what it can do; gets selected; gets called. | Nothing about purpose. Pure capability. | Low-level. The unit of "a thing a Suggestor uses." |

A Suggestor *uses* Backends. A Formation *composes* Suggestors. The engine
sees only Suggestors. Each Suggestor sees only `&dyn Capability` handles
resolved at activation time.

## Why Both Must Exist

Collapse the layers and one of two failure modes follows.

- **Backend-only** (no Suggestor layer): the engine cannot tell who contributed
  what to truth. Authoritative provenance, governed promotion, and the
  convergence loop all degrade into "who shouted last." Axiom 4 dies.
- **Suggestor-only** (no Backend layer): every purposeful unit reimplements
  HTTP, retries, vendor SDKs, capability selection. The kernel re-couples to
  vendor API shape. ADR-007 dies.

The split is what lets a `KnowledgeRetrievalSuggestor` swap LanceDB for Qdrant
without the engine noticing, and lets the engine swap a Suggestor without any
Backend knowing.

## What a Formation Looks Like Under This Rule

A Formation in Organism is a **composition of Suggestors plus a declared set
of capability requirements**. It does not name vendors, and Suggestors do not
name vendors either.

```rust
struct Formation {
    suggestors: Vec<SuggestorId>,
    capabilities: Vec<CapabilityRequirement>,
    invariants: Vec<InvariantId>,
}
```

At activation, Converge resolves `capabilities` against the registered Backend
pool and hands resolved `&dyn Capability` handles to each Suggestor. Any
Backend that satisfies the spec is interchangeable. That gives the
"looks-the-same-from-Converge" property without flattening the layering.

## The Boundary Rule

The sharp edge is the trait visibility line:

- A Suggestor accepts `&dyn ChatBackend` (or whatever capability), never a
  concrete adapter type. It must not import `AnthropicBackend`,
  `OpenAiBackend`, `LanceDbStore`, or any vendor-shaped name.
- A Backend exposes `Capability` declarations and request/response DTOs. It
  must not import `Suggestor`, `Context`, `ProposedFact`, or anything from the
  authoring side.
- The runtime resolves the dyn handle from the registry per-call. That
  registry resolution is where capability matching and selection live.
  Suggestors do not run the registry. Backends do not see it.

## What Hardens Each Layer

**Suggestor contract** ([[Architecture/Ports#Suggestor Execution]]):

- `Send + 'static`-shaped object safety.
- `ContextFact` input is the read-only projection ([[ADRs/ADR-006-promotion-authority-boundary]]).
- `AgentEffect` output is finalized, builder-only.
- `SuggestorId` and ordering rules are stable enough that a Formation can name
  them durably across releases.
- Capability requirements are expressed *declaratively* on the Suggestor, not
  by importing a Backend type.

**Backend / Capability contract** ([[Architecture/Ports#Provider Routing Ports]]):

- Contract crate gets the clean domain name ([[ADRs/ADR-007-provider-tool-contracts]]).
- `BackendRequirements` and `BackendSelector` are the only path from a
  Suggestor's declared need to a usable handle.
- No Backend type leaks into pack-author surfaces.
- Vendor adapters live in extension repositories
  ([[ADRs/ADR-008-extension-crate-boundaries]]).

## Cross-Cutting Implication for v3.8 Extraction

The mnemos / prism / manifold extractions ([[Planning/v3.8 Foundation]]) only
work cleanly because of this rule. Each extension repo implements one or
both layers:

- **mnemos** — implements `KnowledgeRetrievalSuggestor` and `KnowledgeStoreSuggestor`
  (Suggestor layer) on top of vector store and embedding backends (Backend
  layer). Both layers' contracts stay in the foundation; both layers'
  implementations live in mnemos.
- **prism** — implements analytics Suggestors (training, inference, feature
  engineering) on top of Burn / Polars backends. Same pattern.
- **manifold** — primarily a Backend layer extension: HTTP adapters for LLM,
  search, fetch, feed providers. Surfaces capabilities, not purpose. Higher-
  level provider-selection Suggestors may stay in the foundation if they are
  domain-neutral.

If any extension finds itself wanting to expose a type that crosses both
layers, that is a design smell — split the extension along the layer line.

## Anti-Patterns

- A Suggestor with `pub fn new(backend: AnthropicBackend)`.
- A Backend that takes `&dyn Context`.
- A Formation descriptor that names `"openai-gpt-4o"` instead of a capability.
- A pack-author crate that re-exports a vendor adapter for "convenience."
- A Cargo feature on a contract crate that exposes a constructor only the
  engine should hold (the `kernel-authority` mistake).

See also: [[Architecture/Hexagonal Architecture]], [[Architecture/Ports]],
[[Concepts/Formations]], [[Concepts/Backends and Capabilities]],
[[ADRs/ADR-006-promotion-authority-boundary]],
[[ADRs/ADR-007-provider-tool-contracts]],
[[ADRs/ADR-008-extension-crate-boundaries]].
