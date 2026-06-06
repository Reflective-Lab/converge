# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Retired `converge-runtime` as the canonical deployed service. It remains an
  internal, `publish = false`, compatibility-only HTTP/gRPC shell while
  historical scripts and downstream references drain. Runtime Runway now owns
  live app hosting, auth, storage, secrets, telemetry, process lifecycle, and
  deployment; Lattice Mesh owns distributed work; Commerce Rails owns
  commercial authority.

## [3.9.1] - 2026-05-17

### Added
- `converge-kernel` now re-exports `FactPayload`, `ProvenanceSource`, and
  `TextPayload` so embedded applications can implement the 3.9 fact/provenance
  contract from the kernel crate alone.
- `converge-storage::polars_bridge` (gated behind the new `polars` feature):
  shared Parquet read/write surface over any `ObjectStore` backend, with a
  local cache layer keyed by storage path. Lifted out of `crucible-models`
  so every extension that needs columnar data from `file://`, `s3://`,
  `gs://`, or `hf://` consumes a single implementation. Three intent-encoded
  tests pin the load-bearing replay guarantees: typed round-trip,
  cache-on-second-call, and cross-tenant cache key isolation. Pulls
  `parquet 58`, `polars 0.51`, `anyhow`, `tokio`, `tracing` only when the
  feature is enabled — default consumers see no extra deps.
- Workspace `parquet 58` dependency so all extensions pin the same Parquet
  major rather than drifting per-crate.

### Changed
- Release documentation and embedded examples now use typed payloads and
  `*_PROVENANCE.proposed_fact(...)` instead of stale raw string examples.
- Integrated pending dependency updates from `origin/main`.

### Fixed
- Replaced yanked `metrics` 0.24.5 with `metrics` 0.24.6.

## [3.9.0] - 2026-05-15

### Added
- Typed runtime configuration helpers on `ExecutionIdentity`:
  `runtime_config_from_typed<T: Serialize>` and
  `with_runtime_config_typed`.
- `ConvergeError::EmptyProvenance`; fact-emitting suggestors with empty
  `provenance()` now fail inside the engine.
- `FormationKind` in `converge-pack`, re-exported from `converge-core`, plus
  `Context::formation_kind() -> Option<FormationKind>`.
- Canonical provenance markers for the built-in core, kernel, and optimization
  suggestors.

### Changed
- `gate::ProvenanceEnvelope` is now `gate::AuditEnvelope`.
  `ProvenanceEnvelope` remains as a deprecated source-compatible alias.
- `PackSuggestor::provenance()` defaults to the wrapped pack name.
- Reference and test suggestors now declare provenance through
  `ProvenanceSource` markers.

### Dependencies
- Aligns with the latest Mosaic extension floors:
  `converge-arbiter-policy` 2.0.0,
  `converge-prism-analytics` 2.0.0,
  `converge-mnemos-knowledge` 1.2.1,
  `converge-manifold-adapters` 1.1.1,
  `converge-embassy-*` 1.1.1,
  `converge-ferrox-solver` 0.7.0,
  `converge-crucible-models` 0.2.1, and
  `converge-soter-smt` 0.2.1.
- Integrated pending Dependabot bumps from `origin/main`: `async-nats` 0.48,
  `firestore` 0.49, `opentelemetry` / `opentelemetry-otlp` 0.32,
  `ordered-float` 5, `sha2` 0.11, `strum` 0.28, and
  `tracing-opentelemetry` 0.32.

## [3.8.1] - 2026-05-06

First release of the v3.8 line. Foundation has been refocused around the
authority slice; ML, analytics, policy, generic provider adapters, and
domain packs now live in dedicated extension repos.

### Removed
- **BREAKING**: `converge-knowledge` extracted to the **mnemos** extension repo
  (`~/dev/reflective/mosaic-extensions/mnemos`). Foundation no longer ships vector storage,
  agentic memory, or learning implementations. Downstream consumers must
  depend on `mnemos` directly. See ADR-008.
- **BREAKING**: `converge-analytics` extracted to the **prism** extension repo
  (`~/dev/reflective/mosaic-extensions/prism`). Foundation no longer ships Polars/Burn ML
  pipelines, training agents, or analytics packs. Downstream consumers must
  depend on `prism` directly.
- **BREAKING**: `converge-domain` and `examples/*` extracted to the
  **atelier** showcase repo (`~/dev/atelier`). The four built-in domain packs
  (trust, money, delivery, data_metrics) and 13 worked examples now live in
  atelier. Downstream consumers depend on `atelier-domain`.
- **BREAKING**: `converge-policy` extracted to the **arbiter** extension
  repo (`~/dev/reflective/mosaic-extensions/arbiter`). Cedar policy engine, policy suggestors,
  and ed25519-signed delegation tokens now live in arbiter. Foundation
  `converge-pack` keeps the gate trait and authorization vocabulary.
- **BREAKING**: `converge-provider-adapters` (LLM, search, tool, embedding,
  reranker, vector adapters) extracted to the **manifold** extension repo.
  Foundation no longer ships ready-made vendor adapters; downstream
  consumers depend on `manifold` directly.
- 3,469 lines of dead code: orphaned `consensus/` module, broken `billing/`
  module, orphaned `stress_tests.rs`.
- `ContextView` alias (use `Context` directly).

### Changed
- **BREAKING**: Provider contract crate renamed from `converge-provider-api`
  to `converge-provider`. Generic adapter implementations moved to Manifold.
- **BREAKING**: `ContextView` removed — use `Context` (the trait); the
  concrete implementation is now `ContextState`. Consumers using
  `converge_core::ContextView` must change to `converge_core::Context`.

### Added
- **Optimization Suggestor adapter**: `SolverSuggestor<P: Pack>` wraps all
  11 optimization domain packs as first-class Suggestors. Every solver
  participates in the convergence loop via `register_suggestor_in_pack`.
- **Policy Suggestor adapters**: `PolicyGateSuggestor`,
  `DelegationVerifySuggestor`, `FlowGateSuggestor` bridge Cedar policy
  evaluation into the convergence loop.
- **Pack typing primitives**: `UnitInterval` governance primitive,
  `Fact::parse_content` / `ProposedFact::parse_content` typed
  deserialization, `AgentEffect::push` / `extend` for incremental
  composition.
- **Compile-fail contract tests** (7): prove at compile time that wrong
  API usage is impossible — no Fact construction without kernel authority,
  no orphan `Agent` trait, no `register_in_pack`, immutable facts, closed
  `ContextKey` enum.
- **CI/CD infrastructure**: pre-commit hooks, GitHub Actions
  (CI / Security / Coverage / Stability), dependabot auto-merge,
  self-hosted coverage badges across converge, organism, and axiom.
- **700+ new tests** across all modules: property tests, negative tests,
  edge cases, serde roundtrips, async paths with mocked LLM backends.
- `performance-profile` justfile recipe: per-bench targets with optional
  features and explicit `PERF_MODE` (save|compare).

### Fixed
- `[lib] bench = false` on `converge-core` and `converge-optimization` so
  `cargo bench --bench NAME` no longer also runs the lib unittest binary
  in bench mode (which rejects `--save-baseline`).
- Stability workflow: install `protoc` on bench-compile / bench-run / soak
  jobs; remove the dead `live-endpoints` job; bump `cargo-deny-action` to
  v2 (matches the modern config schema); allow `0BSD` license
  (vec_mut_scan via `--all-features`).
- Dependabot auto-merge: drop the approval step. Branch protection on
  `main` does not require reviews, and `GITHUB_TOKEN` is not permitted
  to approve PRs — the step blocked every dependabot PR.

### Dependencies
- `firestore` 0.45.1 → 0.48.0
- `metrics-exporter-prometheus` 0.16.2 → 0.18.3
- `reqwest` 0.12.28 → 0.13.3 (major)
- `toml` 0.8.23 → 1.1.2+spec-1.1.0 (major; tracks TOML 1.1 spec)
- `schneegans/dynamic-badges-action` 1.7.0 → 1.8.0

## [3.7.3] - 2026-04-23

### Added
- Formation template catalog semantics in `converge-model` and the grouped `converge-kernel::formation` surface for downstream formation builders.
- HITL gate decisions as `ExperienceEvent::GateDecisionRecorded`, including experience summaries and store-level query coverage.
- Optional `correlation_id` fields on `FormationDecision` and `FormationOutcome` so upper layers can join formation choices, runs, and outcomes.
- Optional `BackendRequirements` on `ProviderRequest` so formation planners can express role-scoped backend needs through `converge-provider`.
- Organism formation-builder KB notes covering formation compilation, business wedge strategy, and the vendor-selection milestone draft.

### Changed
- `Engine::resume` validates HITL `gate_id` matches before recording a decision or promoting a proposal.
- `converge-kernel` now re-exports the HITL types needed by embedders instead of expanding the non-canonical `converge-core` root surface.
- Provider selection can route a single backend against richer backend requirements when the request supplies them.

### Fixed
- Invalid HITL resume calls now return `ConvergeError::InvalidResume` and map to HTTP `400 Bad Request` in `converge-runtime`.

### Known Limitations
- `StoreObserver` still wraps engine events without tenant or correlation metadata. Formation-level correlation should be handled by an Organism-owned observer until a richer observer contract is justified.
- Current backend selection can enforce backend kind, capabilities, replay, and offline constraints, but not every `BackendRequirements` dimension until backend metadata is expanded.

## [3.4.0] - 2026-04-19

### Added
- **Narrowed downstream surface**: Provider capability contracts (`ChatBackend`, `ModelSelector`, `ChatRequest`/`ChatResponse`) moved from `converge-core` to `converge-provider`. Downstream consumers (`organism`, `helms`, `wolfgang`, `hackathon`) now depend on the lightweight API crate instead of pulling in the full engine.
- **Integrity as contract** (breaking: `ConvergeResult` has new required `integrity` field):
  - `Engine::run` now wraps `Context` in `TrackedContext` internally. Every `add_fact` ticks a Lamport clock and records a SHA-256 content hash.
  - `ConvergeResult.integrity: IntegrityProof` — merkle root, clock time, fact count.
  - `LamportClock` — logical clock for causal ordering without wall clocks.
  - `ContentHash` — SHA-256 content fingerprinting (upgraded from FNV-1a stub).
  - `MerkleRoot` — tamper-evident tree over fact hashes.
  - `TrackedContext` — context wrapper with embedded clock and merkle tree.
  - All exported from `converge_core::integrity` and re-exported via `converge-kernel`.
- **Kong AI Gateway provider**: Full `KongBackend` with Konnect Key Auth, retry with exponential backoff, model selection, and tool calling support. Added to default features.
- **Provider metadata capture**: Generic `metadata: HashMap<String, String>` on `ChatResponse`. Kong captures gateway headers (upstream/proxy latency, request ID, model, rate limits). OpenRouter captures cost, provider, and cost breakdown from response body.
- **`HypothesisResolved` experience event**: New `ExperienceEvent` variant for tracking hypothesis lifecycle across convergence cycles — confirmed, falsified, superseded, or unresolved.
- **Stability Testing Framework**: Production-grade continuous validation pipeline
  - 4 Criterion benchmarks (Engine latency: single-cycle, multi-suggestor cascades 1/5/20, budget pressure, 1000-fact context)
  - 9 chaos tests (panic injection, malformed proposals, latency variance, determinism)
  - 4 soak tests (high-cycle convergence, concurrent runs, throughput latencies, memory stability)
  - 9 property tests documenting Context ID validation gaps (evidence for typed-ID ADR)
  - Automated weekly stability workflow with regression detection (`stability.yml`)
  - Performance baseline tracking and trend analysis (`kb/Baselines/`)

### Changed
- `OutcomeRecorded` experience event now carries optional `metadata: HashMap<String, String>` for provider/gateway telemetry.
- Kong added to provider model registry (gpt-4o, gpt-4o-mini, claude-sonnet-4).
- `crates/application`, `dev/llm`, `dev/docker`, `ops/` moved to `runway` repo. Converge workspace is now focused on the kernel and provider surface.

### Fixed
- Build failure: removed dead `converge_axiom` import and `StubChatBackend` from runtime handlers.
- Clippy: workspace-wide zero-warning pass (runtime, analytics, provider-api, examples).
- Doctests: fixed stale `converge_traits` references in provider-api.
- Integration tests: updated validate_rules tests to match stub endpoint behavior.
- Documentation: all kb/ references to deleted `ops/` paths updated to point to `runway` repo.

### Known Limitations
- `bipartite_matching()` in `converge-optimization` contains a `todo!()` — will panic if called.
- `validate_rules` endpoint is a stub (returns 500). Validation moved to `organism-application`.
- mTLS identity extraction in gRPC interceptor is a placeholder (always returns `None`).
- `rsa` 0.9.10 has a medium-severity advisory (Marvin attack). No upstream fix available; transitive dependency via `jsonwebtoken`.

## [3.2.1] - 2026-04-15

### Changed
- Removed `converge-tool` crate from workspace (moved to separate repository)
- Added file ingest module to `converge-analytics` (CSV, Parquet, Excel)

## [3.2.0] - 2026-04-15

### Added
- **Axiom Layer**: Renamed `converge-tool` to `converge-axiom`. It now serves as the canonical truth layer, providing validation, simulation, and Cedar policy visibility.
- **Async Engine**: `Engine::run` is now a native `async` function. All agent execution and convergence cycles are now runtime-agnostic and non-blocking.
- **OpenRouter Backend**: New `OpenRouterBackend` implementation in `converge-provider` for unified access to 100+ models.
- **Structured Response Formats**: `ResponseFormat` now supports Markdown, YAML, and TOML. YAML is the new default for structured extraction (10% fewer tokens, higher compliance).
- **Resilient Chat**: Added `ResilientChatBackend` with automatic retry, format fallback (YAML -> JSON), and model-based provider fallback.

### Changed
- `JobExecutor` in `converge-runtime` now uses `tokio` tasks and `async` execution instead of `std::thread` and `blocking_recv`.
- Updated ecosystem documentation to the five-layer stack (Helm, Axiom, Organism, Converge, Providers).
- Normalized all `Tracing` spans to use `Instrument` for better async observability.

### Fixed
- Fixed `JobExecutor` streaming buffer and event forwarding in SSE/gRPC handlers.
- Resolved type ownership boundaries in `converge-core` as per ADR-005.

## [3.0.4] - 2026-04-14

### Fixed
- `converge-protocol` now bundles `converge.proto` inside the crate so packaged builds no longer depend on the workspace `schema/` path
- Superseded the partial `3.0.3` crates.io publish with a coherent patch release

## [3.0.3] - 2026-04-14

### Added
- Live endpoint coverage for OpenAI, Anthropic, Gemini, Mistral, Brave, and Tavily, including happy-path and negative credential/model tests
- `MistralBackend` as a live `ChatBackend`
- `BraveSearchProvider` and `TavilySearchProvider` as `WebSearchBackend` implementations

### Changed
- Unified the live provider path on `ChatBackend`, with OpenAI, Anthropic, Gemini, and Mistral using the canonical multi-turn request/response types
- Canonical `ChatMessage` history now carries assistant `tool_calls`, so real tool loops can round-trip without provider-specific patching
- Anthropic tool-call and tool-result history now round-trip against the live API
- Model selection now filters unavailable providers before choosing a backend and routes chat independently from web search

### Fixed
- GitHub Actions now install `protoc` before protobuf builds
- Provider feature builds now include the async runtime dependency they actually use
- Repo docs and KB pages now describe the canonical chat/search split and Anthropic's provider-native JSON behavior correctly

### Removed
- **Knowledge lifecycle pack** (`packs::knowledge`) — moved to `organism-domain`. Includes all knowledge agents (`SignalCaptureAgent`, `HypothesisGeneratorAgent`, etc.), invariants (`ClaimHasProvenanceInvariant`, etc.), and evals (`ClaimProvenanceEval`, `ExperimentMetricsEval`).

## [3.0.2] - 2026-04-11

### Added
- 15 additional proof tests for contract regression gate
- Extract `converge-auth`, `converge-ledger`, `converge-nats`, `converge-consensus` crate boundaries from runtime
- Extract `converge-observability` crate boundary from runtime

### Removed
- All dead code from provider, runtime, knowledge, domain
- OCR, photos, screenshots from `converge-knowledge` (moved to organism)
- OCR source from `converge-provider` (moved to organism-intelligence)
- Billing module from `converge-runtime` (moved to organism)
- Business intelligence moved to organism

## [3.0.1] - 2026-04-10

### Changed
- Hardened runtime and policy control surfaces (fail-closed defaults, auth enforcement, logging)

## [3.0.0] - 2026-04-10

### Added
- **ADR-001**: 6 canonical public crates (`converge-pack`, `converge-provider`, `converge-model`, `converge-kernel`, `converge-protocol`, `converge-client`)
- **ADR-002**: Single truth pipeline (`Observation -> DraftProposal -> ValidatedProposal -> Fact`)
- **ADR-003**: Pack authoring contract (`Suggestor`, `Validator`, `Invariant`, `CriterionEvaluator`, `Pack`)
- **ADR-004**: Contract stabilization and regression gate freeze
- `converge-storage` crate — object storage abstraction (local, S3, GCS)
- `kb/Architecture/System Overview.md` as canonical architecture reference

### Changed
- `converge-traits` deprecated — replaced by `converge-pack` and `converge-provider`
- `converge-remote` demoted to compatibility CLI — replaced by `converge-client`
- Rust edition 2024, rust-version 1.94

## [2.1.2] - 2026-04-08

### Added
- `converge-axiom` publishing enabled for crates.io

## [2.1.1] - 2026-04-07

### Added
- `converge-storage` module and workspace integration
- Embedded Gemma GGUF inference via llama.cpp (`gemma-inference` example)

## [2.1.0] - 2026-04-05

### Added
- Kong AI Gateway provider (`KongGateway`, `KongRoute`)
- CI workflow fixes and stabilization

## [2.0.0] - 2026-04-01

### Added
- **Truth execution framework**: `TruthDefinition`, `TruthKind`, `TruthCatalog` trait
- **Criterion evaluation**: `CriterionEvaluator` trait, four-way `CriterionResult`
- **Pack-scoped execution**: `engine.register_in_pack(pack_id, agent)`, `TypesRootIntent.active_packs`
- `run_with_types_intent_and_hooks()` — single entry point for application-level truth execution
- `StopReason::HumanInterventionRequired` with typed approval references
- `ContextStore` trait for durable context persistence across runs
- `ExperienceEventObserver` for run-scoped event capture
- crates.io publishing enabled for 9 crates

### Changed
- `ProposedFact` now carries `confidence: f64` and typed `provenance: Provenance`
- `AgentEffect` changed from enum to struct `{ facts, proposals }`
- `TryFrom<ProposedFact> for Fact` for type-safe promotion with validation
- Agent trait signatures: `accepts(&self, ctx: &dyn ContextView)` instead of `&Context`
- `converge-traits` is canonical type owner; `converge-core` re-exports
- Copyright updated to Reflective Labs, all SPDX headers normalized to MIT
- Version bumped to 2.0.0 (breaking: ProposedFact, AgentEffect, Agent trait)

### Infrastructure
- Dockerfile, compose.yaml, deploy scripts for GCP Cloud Run
- GitHub CI, security workflows, dependabot

## [1.1.0] - 2024-03-20

### Added
- Initial public release of Converge.zone
- All 15 crates with basic functionality
- CI/CD pipeline configuration
- Documentation and examples

## [1.0.0] - 2024-01-15

### Added
- Initial private development version
- Basic agent runtime
- Core traits and abstractions

[Unreleased]: https://github.com/Reflective-Lab/converge/compare/v3.2.1...HEAD
[3.2.1]: https://github.com/Reflective-Lab/converge/compare/v3.2.0...v3.2.1
[3.2.0]: https://github.com/Reflective-Lab/converge/compare/v3.0.4...v3.2.0
[3.0.4]: https://github.com/Reflective-Lab/converge/compare/v3.0.3...v3.0.4
[3.0.3]: https://github.com/Reflective-Lab/converge/compare/v3.0.2...v3.0.3
[3.0.2]: https://github.com/Reflective-Lab/converge/compare/v3.0.1...v3.0.2
[3.0.1]: https://github.com/Reflective-Lab/converge/compare/v3.0.0...v3.0.1
[3.0.0]: https://github.com/Reflective-Lab/converge/compare/v2.1.2...v3.0.0
[2.1.2]: https://github.com/Reflective-Lab/converge/compare/v2.1.1...v2.1.2
[2.1.1]: https://github.com/Reflective-Lab/converge/compare/v2.1.0...v2.1.1
[2.1.0]: https://github.com/Reflective-Lab/converge/compare/v2.0.0...v2.1.0
[2.0.0]: https://github.com/Reflective-Lab/converge/compare/v1.1.0...v2.0.0
[1.1.0]: https://github.com/Reflective-Lab/converge/releases/tag/v1.1.0
[1.0.0]: https://github.com/Reflective-Lab/converge/releases/tag/v1.0.0
