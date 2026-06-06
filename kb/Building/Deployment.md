---
source: mixed
---
# Deployment

## Direction

Converge no longer targets a standalone hosted `converge-runtime` service as
the canonical deployment path.

Current deployment ownership is:

- Runtime Runway owns app hosting, HTTP/Axum assembly, auth, storage, secrets,
  telemetry, process lifecycle, and Cloud Run deployment.
- Lattice Mesh owns distributed work coordination.
- Commerce Rails owns commercial webhooks, entitlements, receipts, payouts,
  and reconciliation.
- Helm and application backends own product workflows and user-facing runtime
  composition.
- Converge owns the typed kernel and wire contracts those layers embed or
  expose.

See [[Architecture/Runtime Retirement]].

## Compatibility Shell

`crates/runtime` remains in this workspace as a compatibility-only internal
service shell while historical scripts and downstream references drain. It may
still build, expose `/health`, and host old demo/template endpoints, but those
checks prove only that the compatibility shell has not rotted.

Do not add new product, commercial, deployment, or semantic authority to
`converge-runtime`.

## Current Integration Paths

| Need | Use |
|---|---|
| Embed a convergence loop in an app | `converge-kernel` |
| Author suggestors or invariants | `converge-pack` |
| Share semantic model types | `converge-model` |
| Route provider capability | `converge-provider` |
| Expose or consume the wire contract | `converge-protocol` / `converge-client` |
| Run a hosted app/service | Runtime Runway app host or app backend |
| Route live/distributed work | Lattice Mesh |
| Admit commercial state | Commerce Rails |

## Historical Compatibility Commands

These commands are kept only for compatibility maintenance:

```bash
cargo check -p converge-runtime --no-default-features
cargo check -p converge-runtime
cargo test -p converge-runtime --lib
```

Operational commands such as `just dev-up`, `just test-smoke`, and old Cloud
Run deploy scripts should be treated as legacy runtime-shell checks unless a
Runtime Runway or app-host doc explicitly names them as current.
