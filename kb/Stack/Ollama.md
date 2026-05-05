---
tags: [stack, llm]
source: mixed
---
# Ollama

Ollama provides local LLM and embedding inference without cloud dependencies.

## Role in Converge

Ollama is currently a stack option, not a live `converge-provider` `ChatBackend` adapter. The active local inference path is runway's `converge-llm`/Burn (`~/dev/work/runway/crates/llm`). If Ollama is reintroduced as an adapter, it should implement `ChatBackend` and `EmbedBackend` rather than the removed prompt-only contract.

## Configuration

```
Default URL: http://localhost:11434
```

## Supported Models

Any model Ollama supports — Qwen, Llama, Mistral, Phi, Gemma, and more. The provider is model-agnostic.

## Embedding Models

Ollama also provides the `Embedding` port for local vector generation:
- `nomic-embed-text`
- `mxbai-embed-large`

Used with [[Stack/LanceDB]] for fully local semantic search.

## When to Use

- **Data sovereignty** — nothing leaves the network
- **Offline operation** — no internet required
- **Development** — fast iteration without API costs
- **Privacy-sensitive workloads** — PII stays local

## Ollama vs Burn

| | Ollama | [[Stack/Burn]] |
|---|---|---|
| Runtime | External process | In-process |
| Setup | `ollama pull model` | Compile with model weights |
| GPU | Managed by Ollama | Direct CUDA/Metal/CPU |
| Models | Any GGUF | Llama, Gemma, TinyLlama |
| Best for | Development, variety | Production, deterministic replay |

See also: [[Stack/vLLM]], [[Stack/Burn]], [[Architecture/Providers]]
