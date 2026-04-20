---
tags: [architecture]
source: mixed
---
# Crate Map

All crates live under `crates/`. The workspace root `Cargo.toml` centralizes
versions, dependency versions, lints, and build profiles.

## Canonical Public Crates

| Crate | Role |
|---|---|
| `converge-pack` | Pack authoring contract |
| `converge-provider-api` | Provider capability contract |
| `converge-model` | Curated semantic model surface |
| `converge-kernel` | In-process embedding API |
| `converge-protocol` | Generated `converge.v1` wire types |
| `converge-client` | Remote Rust SDK |

## Internal Workspace Crates

| Crate | Role |
|---|---|
| `converge-core` | Engine implementation, promotion gate, integrity tracking |
| `converge-provider` | Provider adapters |
| `converge-domain` | Built-in domain packs and governed flow logic |
| `converge-policy` | Cedar policy engine and policy suggestors |
| `converge-optimization` | Solver packs and `SolverSuggestor` |
| `converge-analytics` | Analytics/ML packs (9: anomaly detection, segmentation, ranking, forecasting, classification, regression, similarity, trend detection, descriptive stats). See [[Analytics Packs]] |
| `converge-knowledge` | Knowledge base and knowledge suggestors |
| `converge-experience` | Experience event storage |
| `converge-runtime` | HTTP and gRPC runtime |
| `converge-storage` | Object storage abstraction |
| `ortools-sys` | Optional OR-Tools FFI bindings |

## Ownership

- `converge-pack` owns the universal authoring contract.
- `converge-provider-api` owns provider capability routing contracts.
- `converge-core` owns the engine implementation and promotion path.
- `converge-kernel` owns the embedding surface above `converge-core`.
- `converge-domain` owns the built-in domain packs.
- `converge-policy`, `converge-optimization`, `converge-analytics`, and `converge-knowledge` all join the same loop by implementing `Suggestor`.

## Publish Reality

Some internal crates are publishable for controlled reuse, but semver promises
apply only to the six canonical public crates.

See also: [[Architecture/API Surfaces]], [[Building/Crate Catalog]]
