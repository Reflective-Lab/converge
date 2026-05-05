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
| `converge-provider-api` | Provider capability contract; transitional name under ADR-007 |
| `converge-model` | Curated semantic model surface |
| `converge-kernel` | In-process embedding API |
| `converge-protocol` | Generated `converge.v1` wire types |
| `converge-client` | Remote Rust SDK |

## Internal Workspace Crates

| Crate | Role |
|---|---|
| `converge-core` | Engine implementation, promotion gate, integrity tracking |
| `converge-provider` | Provider adapters |
| `converge-optimization` | Solver packs and `SolverSuggestor` |
| `converge-experience` | Experience event storage |
| `converge-runtime` | HTTP and gRPC runtime |
| `converge-storage` | Object storage abstraction |

## Ownership

- `converge-pack` owns the universal authoring contract.
- `converge-provider-api` currently owns provider capability routing contracts.
  Its name is transitional; the 3.8 naming rule gives clean domain names to
  contracts and adapter-qualified names to implementations.
- `converge-core` owns the engine implementation and promotion path.
- `converge-kernel` owns the embedding surface above `converge-core`.
- `converge-domain` owns the built-in domain packs.
- `converge-optimization` still joins the convergence loop by implementing `Suggestor`.
- Knowledge suggestors live in **mnemos** (`~/dev/extensions/mnemos`).
- Analytics suggestors live in **prism** (`~/dev/extensions/prism`).
- Policy suggestors and Cedar engine live in **arbiter** (`~/dev/extensions/arbiter`).
- Domain packs and worked examples live in **atelier** (`~/dev/atelier`).
- Provider vendor adapters will live in **manifold** (deferred — see [[Planning/v3.8 Foundation]]).
- See [[ADRs/ADR-008-extension-crate-boundaries]] and [[Architecture/Plug Boundary]].

## Publish Reality

Some internal crates are publishable for controlled reuse, but semver promises
apply only to the six canonical public crates.

See also: [[Architecture/API Surfaces]], [[Building/Crate Catalog]]
