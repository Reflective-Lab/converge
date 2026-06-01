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
| `converge-provider` | Provider capability contract |
| `converge-model` | Curated semantic model surface |
| `converge-kernel` | In-process embedding API |
| `converge-protocol` | Generated `converge.v1` wire types |
| `converge-client` | Remote Rust SDK |

## Internal Workspace Crates

| Crate | Role |
|---|---|
| `converge-core` | Engine implementation, promotion gate, integrity tracking |
| `converge-optimization` | Solver packs and `SolverSuggestor` |
| `converge-experience` | Experience event storage |
| `converge-runtime` | HTTP and gRPC runtime |
| `converge-storage` | Object storage abstraction |

## Ownership

- `converge-pack` owns the universal authoring contract.
- `converge-provider` owns provider capability routing contracts. The 3.8
  naming rule gives clean domain names to contracts and adapter-qualified names
  to implementations.
- `converge-core` owns the engine implementation and promotion path.
- `converge-kernel` owns the embedding surface above `converge-core`.
- Domain packs and worked examples live in **atelier** (`~/dev/atelier`).
- `converge-optimization` still joins the convergence loop by implementing `Suggestor`.
- Knowledge suggestors live in **mnemos** (`~/dev/reflective/mosaic-extensions/mnemos`).
- Analytics suggestors live in **prism** (`~/dev/reflective/mosaic-extensions/prism`).
- Policy suggestors and Cedar engine live in **arbiter** (`~/dev/reflective/mosaic-extensions/arbiter`).
- Source-specific connector ports live in **embassy** (`~/dev/reflective/mosaic-extensions/embassy`).
- Provider vendor adapters live in **manifold**; the in-repo
  `converge-provider-adapters` staging crate was removed after the physical
  move completed (see [[Planning/Manifold Provider Tool Migration]]).
- Storage contracts stay in the foundation; concrete database and object-store
  adapters are extraction candidates. Runtime Runway assembles and operates them.
- See [[Architecture/Extension Topology]],
  [[Architecture/Storage Boundary]],
  [[ADRs/ADR-008-extension-crate-boundaries]], and
  [[Architecture/Plug Boundary]].

## Publish Reality

Some internal crates are publishable for controlled reuse, but semver promises
apply only to the six canonical public crates.

See also: [[Architecture/API Surfaces]], [[Building/Crate Catalog]]
