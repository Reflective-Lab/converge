---
tags: [stack, storage]
source: mixed
---
# SurrealDB

SurrealDB is used as an experience store — persisting events, traces, and artifact state transitions across convergence runs.

## Role in Converge

Implements the `ExperienceStore` [[Architecture/Ports|port]] from outside the
foundation. The v3.8 boundary moves the concrete SurrealDB adapter to
`manifold`.

```rust
SurrealDbConfig {
    url: "ws://localhost:8000",
    namespace: "converge",
    database: "production",
    username: Some("admin"),
    password: Some("secret"),
}
```

## What It Stores

- Experience events from convergence runs
- Artifact state transitions ([[Concepts/Governed Artifacts]])
- Trace links for provenance queries
- Multi-tenant event isolation

## Protocol

WebSocket (`surrealdb::engine::remote::ws::Ws`). Async, persistent connection.

## Why SurrealDB

- Document + relational queries in one engine
- Multi-tenant namespace isolation
- WebSocket protocol fits long-running convergence observability
- Structured query over event metadata

## v3.8 Boundary

SurrealDB is a runtime persistence adapter. Products or Runway wire it into a
running system. Reusable Rust adapter code belongs outside the Converge
foundation; Docker, secrets, volumes, and deployment topology belong in Runway
or product repositories.

See also: [[Architecture/Storage Boundary]], [[Stack/LanceDB]], [[Concepts/Experience and Recall]], [[Architecture/Providers]]
