---
tags: [stack, ml]
source: mixed
---
# Burn

Burn is the neural network framework used for local LLM inference. The
`converge-llm` crate was relocated from the foundation to **Runtime Runway**
(`~/dev/reflective/runtime-runway/crates/llm`) as the local inference distribution
component.

## Role in Converge

Powers the local inference engines — [[Architecture/Providers|providers]] that implement the `LlmBackend` port without network access.

## Backends

| Backend | Hardware | Use Case |
|---|---|---|
| `burn::backend::Wgpu` | Metal (macOS), Vulkan | GPU inference on Apple Silicon and consumer GPUs |
| `burn::backend::CudaJit` | NVIDIA CUDA | Production GPU inference |
| `burn::backend::NdArray` | CPU | Fallback, testing, CI |
| `burn::backend::LibTorch` | PyTorch C++ | When PyTorch models are available |

## Engines Built on Burn

| Engine | Model | Features |
|---|---|---|
| `LlamaEngine` | Llama 3.2 | LoRA adapters, deterministic replay |
| `GemmaEngine` | Google Gemma | GGUF format |
| `TinyLlamaEngine` | TinyLlama | Resource-constrained environments |

## Why Burn

- Pure Rust — no Python runtime, no FFI to PyTorch
- Multi-backend — same model code runs on CUDA, Metal, or CPU
- Deterministic — same input, same output ([[Philosophy/Nine Axioms#6. Transparent Determinism|Axiom 6]])
- LoRA support — fine-tuned adapters without full model copies

## GPU Server

For remote GPU inference, runway's `converge-llm` includes a gRPC server that hosts Burn engines. Clients connect via `GrpcBackend`. This keeps GPU hardware separate from the convergence engine.

See also: [[Stack/Ollama]], [[Stack/vLLM]], [[Architecture/Providers]]
