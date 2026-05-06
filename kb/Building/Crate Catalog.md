---
tags: [building, reference]
source: mixed
---
# Crate Catalog

## Canonical Public Crates

These six crates are the supported external API surfaces.

| Crate | What it does |
|---|---|
| `converge-pack` | Author packs, suggestors, invariants, and proposal-only effects |
| `converge-provider` | Backend identity, capability routing, and chat contracts |
| `converge-model` | Curated semantic types shared across consumers |
| `converge-kernel` | In-process embedding API |
| `converge-protocol` | Generated `converge.v1` wire types |
| `converge-client` | Remote Rust SDK |

## Internal Workspace Crates

These crates are part of the workspace but are not canonical external API
surfaces.

| Crate | What it does |
|---|---|
| `converge-core` | Engine implementation, context state, promotion gate, integrity tracking |
| `converge-provider-adapters` | Temporary non-publishable in-repo provider adapters during Manifold migration |
| `converge-optimization` | Solver packs and `SolverSuggestor` |
| `converge-experience` | Experience event storage |
| `converge-runtime` | HTTP and gRPC runtime |
| `converge-storage` | Object storage abstraction |

Some internal crates are publishable for controlled reuse. That still does not
make them part of the stable public contract. Adapter staging crates such as
`converge-provider-adapters` are not publishable foundation crates.

## Adding a Dependency

For pack authors:

```toml
[dependencies]
converge-pack = "3"
```

For embedded applications:

```toml
[dependencies]
converge-kernel = "3"
converge-model = "3"
```

For remote Rust consumers:

```toml
[dependencies]
converge-client = "3"
```

For provider adapters:

```toml
[dependencies]
converge-provider = "3"
```

ADR-007 records the naming rule: the provider contract owns the clean domain
name, while concrete implementations use adapter-qualified names.

See also: [[Architecture/API Surfaces]], [[Architecture/Crate Map]]
