---
source: mixed
---
# Deployment

## Direction

Converge targets:

- Local native startup for macOS/Linux
- Local container startup with Docker/Podman-compatible compose
- Default hosted path on Google Cloud Run
- Default identity posture using Firebase Auth
- Default infra posture using Terraform + Google Secret Manager
- Optional GPU workers separated from the main runtime

## Reference Implementation

Use `wolfgang-app` as the arrowhead for deployment and auth decisions:

- Frontend: Firebase SDK in `apps/web/src/lib/firebase.ts`
- Frontend auth state: `apps/web/src/lib/stores/auth.svelte.ts`
- Backend auth middleware: `backend/src/http/auth.rs`
- Backend route protection through Axum middleware layers
- Infra root: `ops/infra/environments/prod/wolfgang-bot/main.tf`
- Reusable Cloud Run module: `ops/infra/modules/cloud-run-service/main.tf`
- Firebase Hosting deploy config: see `runway` repo (`ops/deploy/frontend/firebase.json`)
- Operational commands centralized in `Justfile`

## Key Files

- Dev scripts moved to `runway` repo: `dev-up.sh`, `dev-down.sh`, `smoke-test.sh`, `deploy-cloud-run.sh`
- `ops/infra/environments/prod/converge-runtime/main.tf`
- `ops/infra/modules/cloud-run-service/main.tf`, `ops/infra/modules/artifact-registry/main.tf`
- `Dockerfile`, `compose.yaml`
- `crates/provider/.env.example` — provider API key and endpoint catalog

## Verified Facts

These were verified during implementation, not just inferred:

- `converge-runtime` builds in baseline mode
- `converge-runtime` also builds with `--features gcp,auth,firebase`
- `converge-llm-server` builds with `--features server` (now in `runway` repo)
- Native startup via `dev-up.sh native` (now in `runway` repo) can bring up the runtime
- The runtime responds on `/health` once started

## Important Fixes Already Made

1. **Swagger UI removed** — `crates/runtime/src/http.rs` now exposes raw OpenAPI JSON instead of relying on `utoipa-swagger-ui` downloading assets during build
2. **Axum route syntax** — old `:param` routes converted to `{param}` (handlers.rs, pilot.rs)
3. **Native launcher** — `scripts/dev-up.sh` uses `nohup` so the runtime survives after the wrapper exits
4. **GCP/Firebase feature path repaired** — undeclared feature references and missing `rand` dependency fixed in runtime
5. **Rustls provider** — `rustls::crypto::aws_lc_rs::default_provider().install_default()` required for Firestore/rustls startup

## HTTP Auth Posture

- Public routes: `/health`, `/ready`, `/metrics` (when enabled), `/api-docs/openapi.json`
- Protected HTTP routes wrapped in shared Axum auth middleware
- Firebase bearer token validation when built with `gcp,auth,firebase`
- Pilot SSE routes accept `?token=...` for browser EventSource
- Local dev: `DISABLE_AUTH=true` bypass

Key files: `crates/runtime/src/http_auth.rs`, `http.rs`, `handlers.rs`, `pilot.rs`

## Hosted Infra Layout

Terraform environment: `ops/infra/environments/prod/converge-runtime`
Terraform modules: `ops/infra/modules/artifact-registry`, `ops/infra/modules/cloud-run-service`

Hosted flow:
1. Create GCS state bucket
2. `just infra-init`
3. Create required Secret Manager secrets
4. `just infra-apply`
5. `just cloud-build <tag>`
6. `just deploy-runtime <tag>`

## GPU Worker Preparation

GPU scaffolding moved to `runway` repo (`deploy/gpu/cloudrun/`, `deploy/gpu/runpod/`, `deploy/gpu/modal/`).

`converge-llm` server (now in `runway` repo) resolves backend type from compiled features (`cuda`, `wgpu`, fallback CPU).

Still needs: model artifact strategy, secure service-to-service auth, runtime-to-worker call path, deployment test on real GPU infrastructure.

## Known Gaps

1. Firestore is optional in behavior but default in hosted posture — test with no emulator, emulator, and real GCP credentials
2. Terraform is intentionally minimal — no Firebase Hosting, load balancer, custom domains, ingress policy, private auth, production CORS, or multi-env yet
3. GPU worker scaffolding is prepared, not production-complete
4. Runtime has lingering `sentry` feature-flag gate without matching feature declaration

## Resume Commands

```bash
just dev-up
just smoke-test
just dev-down
cargo check -p converge-runtime --features gcp,auth,firebase
cargo check -p converge-llm --bin converge-llm-server --features server
```
