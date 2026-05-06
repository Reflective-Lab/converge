---
tags: [architecture]
source: mixed
---
# Providers

Providers are the adapter implementations that plug into [[Architecture/Ports|ports]]. They live outside the hexagon. The core never imports them — they import the core.

> **Backend-layer plug.** Providers populate the *lower* of the two
> pluggable layers — operational capability adapters. They never see
> `Suggestor`, `Context`, or `ProposedFact`. The Suggestor layer sits above
> them and uses them through capability handles. See
> [[Architecture/Plug Boundary]] for the layering rule.

> **v3.8 location.** Concrete provider implementations move out of the
> Converge foundation into extension repositories (mnemos for knowledge,
> prism for analytics, manifold for generic LLM/search/fetch/feed adapters,
> embassy for source-specific connector ports such as LinkedIn).
> The capability *contracts* stay in the foundation; the *implementations*
> live in extensions. See
> [[Architecture/Extension Topology]],
> [[ADRs/ADR-008-extension-crate-boundaries]] and
> [[Planning/v3.8 Foundation]].

> **Embassy split.** A connector is not automatically a generic provider. If
> the API has to name the external party in its semantic types, such as
> `LinkedInProfile`, it belongs in `embassy`. If the caller should ask for a
> vendor-neutral capability such as web search, feed retrieval, or LLM chat, it
> belongs in `manifold`.

> **Storage split.** Storage is still a port. Runtime persistence stores are
> wired by the host runtime; capability stores/search backends are selected
> through provider routing. Reusable database adapters live in extensions.
> Runway operates databases but does not own the reusable contract. See
> [[Architecture/Storage Boundary]].

## Naming and Location

`converge-provider` is the published provider/capability contract. It owns the
real provider domain name.

Generic provider/tool implementations live in Manifold. The former
`converge-provider-adapters` staging crate has been removed. Generic adapter
implementations carry adapter-qualified names. The clean provider name belongs
to the contract.

Products or Runway supply already-constructed backend handles through
`ChatBackendRegistry`. The registry is a contract surface; adapter crates create
the handles, Converge selects among them.

See [[Planning/Manifold Provider Tool Migration]] for the extraction sequence.

## Chat Backends

### Cloud (Manifold)

Current live remote chat adapters live in Manifold, implement `ChatBackend`,
and are exposed through `DynChatBackend` where runtime polymorphism is needed:

| Backend | Models | Data Sovereignty | Key Capabilities |
|---|---|---|---|
| `AnthropicBackend` | Claude | US | Multi-turn chat, native tool use |
| `OpenAiBackend` | GPT | US | Multi-turn chat, native tool use |
| `GeminiBackend` | Gemini | US | Multi-turn chat, native tool use |
| `MistralBackend` | Mistral | EU | Multi-turn chat, native tool use |

### Response Format Enforcement

All four backends accept `ResponseFormat::Json` on `ChatRequest`. The enforcement mechanism differs by provider:

| Backend | JSON enforcement | Guarantee level |
|---|---|---|
| OpenAI | API flag `response_format: {"type": "json_object"}` | API-enforced |
| Gemini | API field `response_mime_type: "application/json"` | API-enforced |
| Mistral | API flag `response_format: {"type": "json_object"}` | API-enforced |
| Anthropic | System instruction prepend | Instruction-level (provider-native) |

Callers should not assume identical enforcement semantics. Anthropic's instruction-based approach is the documented provider pattern, not a workaround. It is the correct way to request JSON on Anthropic today, but it does not provide the same API-level schema guarantee as providers with a dedicated JSON/response-format field.

Search providers are intentionally separate from chat:

| Backend | Purpose |
|---|---|
| `BraveSearchProvider` | Web search with snippet retrieval |
| `TavilySearchProvider` | Web search with answer/raw-content support |

`Brave` and `Tavily` are not `ChatBackend`s. They are search tools that can be composed with chat backends at the workflow layer.

### Local Inference (runway `converge-llm`)

Local inference lives in runway (`~/dev/work/runway/crates/llm`) — it is
the local-inference distribution component, not a foundation contract:

| Engine | Framework | GPU Support | Use Case |
|---|---|---|---|
| `LlamaEngine` | llama-burn | CUDA, Metal (Wgpu), CPU (NdArray) | Llama 3.2, LoRA adapters |
| `GemmaEngine` | Burn | CUDA, Metal, CPU | Google Gemma GGUF |
| `TinyLlamaEngine` | Burn | CPU | Resource-constrained environments |
| `GrpcBackend` | Tonic client | Remote GPU | Offload to GPU server |

## Storage Providers

Storage providers implement runtime persistence ports or capability backends.
The implementation crates should live outside the foundation unless they are
small deterministic test support.

| Provider | Protocol | Best For |
|---|---|---|
| SurrealDB adapter | WebSocket | Multi-tenant, relational + document queries |
| LanceDB adapter | Local/remote | Vector-indexed retrieval, similarity search |
| `InMemoryExperienceStore` | None | Development, testing |

Object stores (S3, GCS, local filesystem) are artifact persistence adapters;
their SDKs and credentials should not define the Converge foundation release
cadence.

## Search & Embedding Providers

| Provider | Port | Purpose |
|---|---|---|
| Brave | `WebSearchBackend` | Web search snippets |
| Tavily | `WebSearchBackend` | Web search with answer/raw content |
| LanceDB vector | `VectorRecall` | Similarity search over embeddings |
| Qdrant | `VectorRecall` | Managed vector search |
| FastEmbed | `Embedding` | Local vector embedding generation |
| Ollama embeddings | `Embedding` | Local embedding via nomic-embed, mxbai-embed |
| Cloud embeddings | `Embedding` | OpenAI, Hugging Face embedding APIs |
| Cross-encoder | `Reranking` | Re-rank search results by relevance |

## Optimization Providers

| Provider | Port | Purpose |
|---|---|---|
| Native optimization packs | `Suggestor` / pack solver | Scheduling, routing, resource allocation, multi-criteria optimization |
| Varisat SAT | Constraint solver | Optional native constraint programming |

## Capability Presets

```rust
BackendRequirements::fast_llm()           // Low cost, fast response
BackendRequirements::reasoning_llm()      // High cost, reasoning-capable
BackendRequirements::access_policy()      // Policy engine
BackendRequirements::constraint_solver()  // Optimization
BackendRequirements::embedding_pipeline() // Vector embeddings
BackendRequirements::vector_search()      // Similarity recall
```

## The Adapter Rule

Providers produce **observations, never decisions** ([[Philosophy/Nine Axioms#4. Agents Suggest, Engine Decides|Axiom 4]]). A `ChatBackend` response or `WebSearchBackend` result becomes input to an agent or workflow. The engine's promotion gate decides what becomes truth. Providers have no governance authority.

See also: [[Architecture/Plug Boundary]], [[Architecture/Ports]], [[Architecture/Storage Boundary]], [[Architecture/Hexagonal Architecture]], [[Concepts/Backends and Capabilities]]
