---
tags: [architecture, decision]
source: codex
---
# Runtime Retirement

## Decision

As of 2026-06-02, `converge-runtime` is retired as the canonical deployed
service for Converge.

Converge owns the typed convergence kernel and public contracts:

- `converge-pack`
- `converge-provider`
- `converge-model`
- `converge-kernel`
- `converge-protocol`
- `converge-client`

It does not own the live service control plane for the Reflective stack.
Runtime Runway owns app hosting, transport defaults, auth, storage, secrets,
telemetry, process lifecycle, and deployment. Lattice Mesh owns distributed
work coordination. Commerce Rails owns commercial authority. Helm and the
applications own product surfaces and module composition.

## Current Status

`crates/runtime` remains in the workspace as a compatibility-only internal
service shell while downstream references and historical scripts are drained.
It is `publish = false`, it is not a public API surface, and new product or
platform semantics must not be added there.

The compatibility crate may still be checked to keep the workspace honest, but
passing `converge-runtime` checks does not mean the Reflective stack is
deployed through it.

## Ownership Move

| Concern | Retired home | Current owner |
|---|---|---|
| HTTP/Axum service shell | `converge-runtime` | Runtime Runway app host and app backends |
| gRPC service process | `converge-runtime` | Runtime Runway or Lattice Mesh, per deployment |
| SSE/live transport | `converge-runtime::sse` | Runtime Runway `EventHub` / app host |
| Auth middleware | `converge-runtime::{auth,http_auth}` | `runway-auth` and app-level policy |
| Secrets | `converge-runtime::secrets` | `runway-secrets` |
| Storage adapters | `converge-runtime::{db,gcp}` | `runway-storage` and app-owned stores |
| Telemetry/metrics | `converge-runtime::{telemetry,metrics}` | `runway-telemetry` |
| Job/template demo endpoints | `converge-runtime::{handlers,templates}` | Helm, Atelier, and thin apps |
| Commercial webhooks and entitlements | runtime handlers | Commerce Rails plus Runtime Runway ingress |
| Distributed worker coordination | runtime process | Lattice Mesh |
| Convergence loop | unchanged | `converge-kernel` / `converge-core` |

## Deletion Gates

Do not delete `crates/runtime` until all gates are true:

1. `converge.zone` no longer rewrites `/api/**` to Cloud Run service
   `converge-runtime`.
2. Runtime Runway local Docker/dev scripts either target app-host services or
   are explicitly marked as legacy compatibility scripts.
3. No downstream repo imports `converge_runtime` as a dependency.
4. CI and Justfile recipes no longer treat `converge-runtime` as the service
   that proves stack deployment.
5. Historical docs keep their chronology, while current architecture docs
   clearly point to Runtime Runway, Lattice Mesh, Commerce Rails, Helm, and app
   hosts.

## Rules While It Remains

- No new semantic authority belongs in `converge-runtime`.
- No new application workflow belongs in `converge-runtime`.
- No new commercial policy belongs in `converge-runtime`.
- No public semver promises attach to `converge-runtime`.
- Prefer typed contracts in public crates or Runtime Runway crates over string
  conventions in the compatibility shell.

See also: [[Architecture/API Surfaces]], [[Architecture/Crate Map]],
[[Architecture/System Overview]], and
`~/dev/reflective/KB/04-architecture/runtime-injection-boundaries.md`.
