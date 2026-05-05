---
tags: [concepts]
source: mixed
---
# Backends and Capabilities

Backends are the first-class instantiation types in Converge. No backend kind is privileged — an LLM provider is no more important than an optimizer or a policy engine.

## Backend Trait

```rust
pub trait Backend: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> BackendKind;
    fn capabilities(&self) -> Vec<Capability>;
    fn provenance(&self, request_id: &str) -> String;
    fn has_capability(&self, cap: Capability) -> bool;
    fn supports_replay(&self) -> bool;
    fn requires_network(&self) -> bool;
}
```

## Backend Kinds

| Kind | Examples |
|---|---|
| `Llm` | Claude, GPT, Qwen, Ollama |
| `Policy` | Cedar, OPA |
| `Optimization` | Native CP, SAT, graph optimization |
| `Analytics` | Burn, Polars |
| `Search` | Vector, full-text |
| `Storage` | Firestore, PostgreSQL |
| `Other(String)` | Extensibility |

## Capability-Driven Selection

Selection is by capability, not by name. Don't choose "use Claude" — choose "I need TextGeneration + Reasoning." The selector finds a backend that satisfies those capabilities.

70+ capabilities are declared across categories: generation, policy, optimization, analytics, search, storage.

## Providers

Providers are adapters that wrap external systems and implement the Backend trait. They produce **observations, never decisions** ([[Philosophy/Nine Axioms#4. Agents Suggest, Engine Decides|Axiom 4]]). A provider response becomes an Observation, which an agent may use to create a ProposedFact, which the engine may promote to a Fact.

See also: [[Building/Crate Catalog]], [[Concepts/Proposals and Promotion]]
