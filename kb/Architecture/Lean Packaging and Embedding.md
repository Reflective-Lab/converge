---
tags: [architecture, packaging, embedding]
source: mixed
---
# Lean Packaging and Embedding

Converge has two very different consumers:

- embedders that want the kernel and little else
- service operators that want the full runtime shell

Those should not pay the same dependency and binary-size cost.

## Public Rule

- use `converge-kernel` for in-process embedding
- use `converge-runtime` only when you actually need the HTTP / gRPC service shell
- keep the runtime shell's convenience dependencies out of the embedding path

## Current Packaging Variants

The current baseline is measured with:

```bash
just size-audit
```

That recipe audits:

- `converge-runtime --release --no-default-features` as `minimal`
- `converge-runtime --release` as `standard`
- `converge-runtime --release --all-features` as `full`
- `converge-kernel --release --lib` as the embedding artifact

### Initial 2026-04-22 Baseline

From the first `just size-audit` run:

- `converge-runtime --release --no-default-features`: `7.87 MiB`

That is already enough to show the current problem: the so-called minimal
runtime shell is still carrying a lot of baggage for an embedder-friendly
story.

### 2026-05-18 Baseline

Measured from branch `measure/packaging-baseline` with:

```bash
just size-audit
```

Artifact sizes:

| Variant | Command | Result |
|---|---|---|
| runtime minimal | `cargo build -p converge-runtime --release --no-default-features` | `8,289,904` bytes / `7.91 MiB` |
| runtime standard | `cargo build -p converge-runtime --release` | `10,030,608` bytes / `9.57 MiB` |
| runtime full | `cargo build -p converge-runtime --release --all-features` | **build-fail** |
| kernel | `cargo build -p converge-kernel --release --lib` | `328,112` bytes / `0.31 MiB` |

Dependency graph shape, counted as unique `cargo tree --prefix none` lines:

| Surface | Unique lines |
|---|---:|
| runtime minimal | `398` |
| runtime standard | `425` |
| runtime full | `567` |
| kernel | `99` |

Disk footprint after the audit:

| Path | Size |
|---|---:|
| `/tmp/converge-size-audit` | `1.9G` |
| `/tmp/converge-size-audit/release/deps` | `1.7G` |
| workspace `target/` | `15G` |

The failed full build is itself part of the baseline. Enabling all runtime
features exposed bitrot in several optional surfaces:

- `security` references missing `x509_parser` and `oid_registry` dependencies
- `billing` references a missing `crate::billing` module
- `telemetry` is incompatible with the current OpenTelemetry dependency set and
  pulls conflicting OpenTelemetry versions into the graph
- `wasm` belonged above Converge's runtime boundary and pulled Wasmtime/Cranelift
  into the foundation graph

The biggest release-deps artifacts observed during the failed full attempt were
Wasmtime/Cranelift, `async-nats`, GCP/Firestore, duplicate transport stacks, and
OpenTelemetry. That matches the current user concern: the runtime shell pulls in
implementation-heavy surfaces too early, while `converge-kernel` remains small
enough for the embedding story.

### 2026-05-18 Observability Dependency Gate

The first dependency-gating cut made the existing `metrics` and `telemetry`
feature names own their heavy dependencies instead of compiling them
unconditionally.

Measured effect:

| Surface | Before | After | Delta |
|---|---:|---:|---:|
| runtime minimal dependency lines | `398` | `352` | `-46` |
| runtime standard dependency lines | `425` | `383` | `-42` |
| runtime minimal artifact | `7.91 MiB` | `7.91 MiB` | no linked-size change |
| runtime standard artifact | `9.57 MiB` | `9.57 MiB` | no meaningful linked-size change |

This is a compile/disk blast-radius improvement, not a linked-binary-size
improvement. It removes Prometheus and OpenTelemetry from minimal/default
runtime dependency graphs, but the runtime binary still links enough HTTP,
auth, TLS, request, NATS, and shell machinery that the final executable size
does not move yet.

Verification:

- `cargo check -p converge-runtime --no-default-features`
- `cargo check -p converge-runtime`
- `cargo check -p converge-runtime --features metrics`
- `cargo check -p converge-runtime --no-default-features --features metrics`
- `just size-audit`

Remaining observability split:

- `tracing` and `tracing-subscriber` are still unconditional because the runtime
  binary initializes logging directly in `main.rs`
- `telemetry` now compiles when enabled, but remains a heavyweight optional
  runtime adapter

### 2026-05-18 Full Runtime Feature Repair

The full runtime feature set is now measurable instead of failing during
compilation.

Changes:

- removed the stale `billing` runtime feature and Stripe wiring; Commerce Rails
  owns billing semantics and provider-specific Stripe state outside this runtime
- removed the stale `sentry` feature stub rather than adding a new heavy
  reporting stack to make an unused flag compile
- aligned OpenTelemetry dependencies with `tracing-opentelemetry` so telemetry
  uses one compatible OpenTelemetry API family
- removed the X.509 parser dependency expectation from file identity loading;
  explicit service identity remains configuration-owned
- repaired stale WASM/core type conversions long enough to measure the full
  runtime graph before moving the plugin runtime to Helm

Measured result:

| Variant | Command | Result |
|---|---|---|
| runtime minimal | `cargo build -p converge-runtime --release --no-default-features` | `8,251,648` bytes / `7.87 MiB` |
| runtime standard | `cargo build -p converge-runtime --release` | `10,010,032` bytes / `9.55 MiB` |
| runtime full | `cargo build -p converge-runtime --release --all-features` | `15,417,280` bytes / `14.70 MiB` |
| kernel | `cargo build -p converge-kernel --release --lib` | `328,112` bytes / `0.31 MiB` |

Dependency graph shape:

| Surface | Unique lines |
|---|---:|
| runtime minimal | `352` |
| runtime standard | `383` |
| runtime full | `564` |
| kernel | `100` |

Disk footprint after the audit:

| Path | Size |
|---|---:|
| `/tmp/converge-size-audit` | `2.3G` |
| `/tmp/converge-size-audit/release/deps` | `2.0G` |
| workspace `target/` | `21G` |

Verification:

- `cargo check -p converge-runtime --no-default-features`
- `cargo check -p converge-runtime --all-features`
- `cargo test -p converge-runtime --all-features`
- `just check`
- `just lint`
- `just size-audit`

### 2026-05-18 WASM Plugin Runtime Moved to Helm

Sandboxed WASM execution is no longer a `converge-runtime` feature. Helm owns
the application plugin runtime because plugin lifecycle, module signing, tenant
quotas, and executable app extensions are product concerns above the Converge
foundation.

Converge keeps the typed convergence contracts and kernel boundary. Helm may
adapt a verified plugin into Converge proposals or invariants, but Converge does
not embed Wasmtime or expose a WASM feature flag.

The cross-layer agreement is:

| Layer | Responsibility |
|---|---|
| Axiom | Produce WASM-facing invariant artifacts, manifests, hashes, lineage, replay metadata, and proof obligations from a Truth Package. |
| Helm | Host executable plugins in `helm-plugin-runtime`, enforce signing/quota/host-call policy, and adapt sandbox output into Converge-facing contracts. |
| Converge | Recompute authority, enforce promotion gates and stop reasons, attach evidence refs and trace links, and record run integrity. |

Plugin execution is never promotion. A successful sandbox run can produce
evidence, a proposal, or an invariant verdict; Converge still decides whether
anything becomes governed fact.

Measured effect:

| Surface | Before | After | Delta |
|---|---:|---:|---:|
| runtime full dependency lines | `564` | `454` | `-110` |
| runtime full artifact | `15,417,280` bytes / `14.70 MiB` | `14,886,384` bytes / `14.20 MiB` | `-530,896` bytes / `-0.50 MiB` |

The minimal and standard runtime artifacts did not move because WASM was already
outside those feature sets. The full feature set now excludes Wasmtime/Cranelift
from Converge entirely.

Verification:

- `cargo check -p converge-runtime --all-features`
- `just check`
- `just lint`
- `just size-audit`

### 2026-05-18 Serialization Boundary Cut

The first serialization pass deliberately avoided making `serde` optional
across the public crates. In this workspace, serialization is not only a
transport convenience:

- `converge-pack::FactPayload` requires `serde::Serialize`
- core fact, proposal, provider, and formation DTOs derive `Serialize` /
  `Deserialize`
- several public payload fields intentionally use `serde_json::Value` as the
  open boundary representation

The safe cleanup was therefore limited to dependencies that are not part of a
normal public contract:

- moved `converge-experience`'s `serde_json` dependency to dev-dependencies
  because it is used only by test roundtrips
- kept `serde` in `converge-kernel`; provider-selection payloads implement
  `FactPayload`, which requires `Serialize`

Verification:

- `cargo check -p converge-experience`
- `cargo test -p converge-experience --lib`
- `cargo check -p converge-kernel`

## Current Reality

The old milestone wording talks about `converge-application`. That crate no
longer exists in this workspace. The service shell that needs trimming now is
`converge-runtime`.

## First v3.4 Cut

The first lean-packaging cleanup removed dead direct dependencies from
`converge-runtime` and later pruned their stale workspace entries:

- `clap`
- `config`
- `dotenv`

Those were inflating the runtime dependency graph without owning any real
behavior in the current shell.

As of 2026-05-18, `config` and `dotenv` are not present in the workspace
dependency graph. `clap` still appears only through Criterion dev tooling, not
as a Converge runtime dependency.

## Scope

This work should be broken into five deliberately boring slices.

### Slice 1 — Baseline and Tooling

Goal:
- make the current cost visible and repeatable

Deliverables:
- `just focus`, `just sync`, and `just status` match the documented workflow
- `just size-audit` produces repeatable artifact-size numbers
- this page records the baseline

### Slice 2 — Dependency Trim

Goal:
- remove dead or gratuitous dependencies from `converge-runtime`

Current examples:
- `clap`
- `config`
- `dotenv`

Acceptance:
- `cargo tree -p converge-runtime` shrinks for unchanged behavior
- `cargo check -p converge-runtime`
- `cargo check -p converge-runtime --no-default-features`

### Slice 3 — Feature-Gate the Runtime Shell Honestly

Goal:
- make `minimal` actually minimal

Current problem:
- the current `--no-default-features` build still compiles substantial
  runtime-shell baggage because many dependencies are unconditional even when
  their modules are feature-gated

Targets:
- telemetry stack
- metrics exporter
- NATS client
- gRPC / protocol stack
- GCP-specific runtime adapters
- auth / security-only wiring that is not part of the minimal shell

Acceptance:
- `minimal` build no longer compiles telemetry, NATS, or gRPC machinery unless
  explicitly enabled
- the `just size-audit` delta is visible and documented

### Slice 4 — Packaging Variants

Goal:
- define one boring packaging matrix and stick to it

Proposed variants:
- `kernel`
  - `converge-kernel` only
  - for Wolfgang / Organism style embedding
- `minimal`
  - HTTP runtime shell, no gRPC, no telemetry, no NATS, no cloud-specific
    runtime adapters
- `standard`
  - current default server story
- `full`
  - everything on

Acceptance:
- each variant has one documented build invocation
- each variant has one measured size
- docs stop speaking in aspirations and start speaking in actual commands

### Slice 5 — Downstream Proof

Goal:
- prove the trimmed surfaces serve real sibling repos

Proof points:
- Wolfgang embedding should use `converge-kernel`, not `converge-runtime`
- runtime consumers that truly need the service shell should be able to choose
  `minimal` or `standard` intentionally

Acceptance:
- one downstream check in Wolfgang and/or Organism
- one short embedding guide that reflects the real boundary

## Ownership Boundary

### Stays In Converge

- kernel semantics
- governed experience/audit semantics
- runtime modules that are genuinely reusable server adapters
- optional observability and audit adapters if they remain part of the supported
  runtime shell

### Does Not Belong In Converge

- deleted `ops/` shell scripts and dev distribution scaffolding
- deployment recipes and environment management that are already in `runway`
- product-specific runtime bootstrapping

### Not Decided Yet

Audit and telemetry code are still in `converge-runtime` today. The immediate
v3.4 job is **not** to throw them into `runway` by reflex. The immediate job is
to:

1. make them optional in the runtime shell
2. measure the impact
3. decide later whether they deserve their own runtime-adapter crate in a
   surface-reduction pass

That is a v3.6-style split question, not the first cut of v3.4.

## What Comes Next

1. Gate NATS, cloud, and auth/security dependencies more tightly so minimal
   stays boring.
2. Decide whether telemetry/audit deserve a later runtime-adapter crate split.
3. Write the embedding guide around `converge-kernel`.
