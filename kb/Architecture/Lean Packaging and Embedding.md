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
features currently exposes bitrot in several optional surfaces:

- `security` references missing `x509_parser` and `oid_registry` dependencies
- `billing` references a missing `crate::billing` module
- `telemetry` is incompatible with the current OpenTelemetry dependency set and
  pulls conflicting OpenTelemetry versions into the graph
- `wasm` has stale API/type mismatches and missing `hex` wiring

The biggest release-deps artifacts observed during the failed full attempt were
Wasmtime/Cranelift, `async-nats`, GCP/Firestore, duplicate transport stacks, and
OpenTelemetry. That matches the current user concern: the runtime shell pulls in
implementation-heavy surfaces too early, while `converge-kernel` remains small
enough for the embedding story.

## Current Reality

The old milestone wording talks about `converge-application`. That crate no
longer exists in this workspace. The service shell that needs trimming now is
`converge-runtime`.

## First v3.4 Cut

The first lean-packaging cleanup removed dead direct dependencies from
`converge-runtime`:

- `clap`
- `config`
- `dotenv`

Those were inflating the runtime dependency graph without owning any real
behavior in the current shell.

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

1. Fix or quarantine the broken `--all-features` runtime surfaces so `full`
   becomes measurable.
2. Gate telemetry, metrics, NATS, cloud, and WASM dependencies honestly.
3. Re-run `just size-audit` and compare deltas.
4. Write the embedding guide around `converge-kernel`.
