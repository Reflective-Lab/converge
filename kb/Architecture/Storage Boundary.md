---
tags: [architecture, storage, ports, extensions]
source: mixed
date: 2026-05-05
---
# Storage Boundary

Storage is a hexagonal port, not a third plugin category.

Converge distinguishes two storage roles:

1. **Runtime persistence** — state the host runtime wires into the engine:
   experience events, context snapshots, trace links, artifacts, and blobs.
   Formations do not select this. The embedder or deployment config chooses it.
2. **Capability storage/search** — operational storage a Suggestor asks for:
   vector recall, document search, semantic search, or knowledge retrieval.
   This is selected through backend/capability routing like any other
   operational provider.

## Decision

Universal storage contracts may stay in the Converge foundation. Concrete
database implementations move out to extensions or product/deployment layers.

```text
Converge storage contracts <- storage adapters <- runway / products
```

Converge must not depend on SurrealDB, LanceDB, S3/GCS SDKs, or product
database wiring in its long-lived foundation surface. Those dependencies have
their own release cycles, security patches, native build costs, and deployment
constraints.

## What Goes Where

| Concern | Home | Reason |
|---|---|---|
| `ExperienceStore`, context snapshot, artifact/blob contracts | Converge foundation | Universal runtime persistence ports. |
| In-memory store for tests and examples | Foundation or test support | Small, deterministic, no external service dependency. |
| SurrealDB implementation | Extension adapter, assembled by Runway/products | Heavy runtime persistence implementation. |
| LanceDB knowledge/recall implementation | `mnemos` | Knowledge and vector recall are mnemos' domain. |
| Generic vector search adapter | `manifold` unless it becomes mnemos-specific | Search capability hidden behind backend requirements. |
| S3/GCS/local object store adapters | Extension adapter crates | SDK and credential churn should not define Converge release cadence. |
| Docker compose, service startup, secrets, volumes, migrations | `runway` or product repos | Operational assembly, not reusable contract ownership. |

## Runway Rule

Runway is the right place to **operate** heavy databases. It owns Docker,
processes, deployment topology, persistent volumes, secrets, environment
selection, and runtime packaging.

Runway is not the right upstream home for reusable Rust storage contracts or
adapters. Runway consumes extension crates and assembles them into runnable
systems; it does not become a dependency of Converge, Organism, or Helm just so
they can use a storage implementation.

## Boundary Tests

- If a store persists Converge runtime state, it is a runtime persistence port.
  The host wires it in.
- If a Suggestor asks for "semantic recall" or "document search", it is a
  backend/capability provider.
- If the implementation imports a database SDK, cloud SDK, network client, or
  credential model, it belongs outside the foundation.
- If it only defines typed request/response values and traits, it can be a
  foundation contract.

## Extraction Status

Closed for v3.8. `converge-storage` now carries object-storage contract types
only. `converge-experience` carries in-memory test support and observer
plumbing only. Concrete local/S3/GCS object-store builders, SurrealDB and
LanceDB experience stores, and the generic LanceDB vector adapter live in
`~/dev/extensions/manifold-adapters`.

Prism consumes only the `converge-storage` object-store contract; it no longer
depends on foundation-owned local storage builders. Runway remains the place to
operate databases and wire credentials, not the reusable Rust adapter home.

See also: [[Architecture/Extension Topology]], [[Architecture/Ports]],
[[Architecture/Providers]], [[Planning/v3.8 Foundation]].
