# Milestones

> See `~/dev/work/EPIC.md` for the coarse-grained outcomes these milestones advance.

> Architecture follow-up for crate boundaries: [[Planning/CRATE-BOUNDARY-REALIGNMENT]].

## Backlog: OR-Tools Native Solver Wiring
**Epic:** E1 (optimization capability)

Wire the existing `ortools-sys` FFI bindings into the CP and VRP pack solvers to replace the current backtracking and nearest-neighbor heuristics with production-grade solvers.

- [ ] Wire CP-SAT into `constraint_programming` pack solver (replaces `BacktrackingSolver`)
- [ ] Wire OR-Tools routing library into `vehicle_routing` pack solver (replaces `NearestNeighborVrpSolver`)
- [ ] Add `brew install or-tools` / CI image requirement to dev setup docs
- [ ] Add reference validation tests against published CP-SAT and VRP benchmarks
- [ ] Ensure `ortools-sys` feature-gates so binaries without OR-Tools still link

**Why this matters:**
- Backtracking solver blows up exponentially for n > ~10 variables
- Nearest-neighbor VRP produces routes 20–40% above optimal with no time-window support
- OR-Tools CP-SAT and LNS give practical scale (hundreds of variables, 50+ stops) without writing new algorithms
- FFI bindings already exist; this is integration work, not research

**Prerequisite:** OR-Tools shared library available at link time (not currently in CI)

## Planned: v3.4 — Lean Packaging & Embedding
**Target:** 2026-06-15 | **Epic:** E1, E3 (Wolfgang/Organism embedding)

**Theme:** Enable lightweight embedding in Wolfgang/Organism by separating dev distribution from production binaries. Rust's feature gates + workspace structure make this solvable.

- [ ] Audit binary sizes: measure what each crate contributes (converge, converge --features full, kernel API)
- [ ] Feature-gate observability: tracing, tracing-subscriber → optional (not required for lean services)
- [ ] Feature-gate configuration: config, dotenv → optional (embed-friendly: read from env only)
- [ ] Feature-gate serialization: serde, serde_json → optional (only needed for API boundaries)
- [ ] Refactor converge-application: separate dev defaults from runtime essentials
- [ ] Define packaging variants: kernel (lib), minimal (binary ~2MB), standard (~5MB), full (~50MB+)
- [ ] Document embedding guide: "Use converge-kernel for Wolfgang; use minimal binary for standalone"
- [ ] Test Wolfgang integration: verify zero-overhead embedding with feature gates
- [ ] Publish with release notes highlighting binary size improvements

**Why this matters:**
- Wolfgang embeds Converge via kernel API; shouldn't pay for tokio, observability, CLI
- Current application defaults are "rich" (good for dev, wasteful for services)
- Rust features solve this cleanly; leverage them for clean builds

## Planned: v3.5 — Capability Contract Realignment
**Target:** 2026-07-15 | **Epic:** E1, E2, E3

**Theme:** Make `converge-provider-api` the live provider and capability contract so sibling repos can consume stable capability types without importing `converge-core` internals.

- [ ] Promote `converge-provider-api` from identity-only contract to the canonical capability surface
- [ ] Move or re-home chat and embed traits, request and response types, and selection vocabulary under one canonical owner
- [ ] Collapse duplicate provider taxonomies across `converge-core`, `converge-provider-api`, and `converge-provider`
- [ ] Keep `converge-core` focused on engine, truth pipeline, governance, and deterministic execution
- [ ] Migrate `axiom` off direct `converge-core` imports for capability-facing concerns
- [ ] Add regression proof that downstreams can select and call providers without importing `converge-core`
- [ ] Update API documentation to reflect the live boundary rather than the aspirational one

**Why this matters:**
- `organism` validates the kernel boundary, but `axiom` proves the provider boundary is still wrong
- a stable capability contract is required before Wolfgang, Organism, and Axiom can share long-lived integration assumptions
- one owner per public concept is the only durable way to prevent the next round of type drift

## Planned: v3.6 — Surface Reduction & Service Split
**Target:** 2026-08-15 | **Epic:** E1, E3

**Theme:** Make publishability match support intent and keep reusable libraries separate from service shells and transport stacks.

- [ ] Classify every currently publishable crate as canonical, transitional, or private
- [ ] Set `publish = false` on crates that are not deliberately supported external contracts
- [ ] Split library concerns from server and CLI concerns in crates such as `converge-policy` and `converge-knowledge`
- [ ] Remove unnecessary provider coupling from library crates where trait boundaries already suffice
- [ ] Add a CI guard that fails on accidental expansion of the publishable surface
- [ ] Reconcile `Cargo.toml` manifests with `kb/Architecture/API Surfaces.md` and `kb/Architecture/Crate Map.md`

**Why this matters:**
- six canonical public crates and sixteen publishable crates is operationally incoherent
- library consumers should not inherit HTTP, gRPC, auth, or CLI baggage by accident
- a smaller supported surface improves long-term maintenance and security review quality

## Planned: v3.7 — Curated Facades & Downstream Proof
**Target:** 2026-09-15 | **Epic:** E1, E2, E3

**Theme:** Make `converge-kernel`, `converge-model`, and `converge-provider-api` the obvious consumer entry points and prove them against real sibling repos.

- [ ] Curate `converge-kernel` so the embedding API is deliberate rather than a raw re-export dump
- [ ] Curate `converge-model` so downstreams read governed semantic types without internal `Types*` leakage
- [ ] Add or automate downstream proof against `organism` and `axiom`
- [ ] Deprecate remaining direct downstream `converge-core` usage where canonical contracts now exist
- [ ] Publish release notes describing the final supported surface and migration guidance

**Why this matters:**
- stable facades are only real when they are easier to use than the internals
- sibling repos are the best proof that the architecture is serving the ecosystem, not just the current workspace
- this is the point where the documented contract becomes operational truth

## Current: v3.3 — Contract Enforcement
**Deadline:** 2026-05-10 | **Epic:** E1 (Converge is a publishable platform)  
**Status:** ✅ **Completed** (2026-04-17)

- [wontfix] ~~Add `#[warn(missing_docs)]` to all publishable crates~~ — docs belong in kb/, not in code
- [x] Migrate async traits: ChatBackend/EmbedBackend adoption across providers
- [x] Canonize ExperienceStore as the live trait (removed stale #[deprecated], added test)
- [x] Write ADR-005: Type ownership boundaries (which crate owns which types)
- [x] Export CriterionResult/CriterionOutcome/Criterion from converge-kernel

### Scope Drift (2026-04-17): Stability Testing Framework
**Out-of-scope for v3.3 but high-impact foundation work:**
- Implemented 6-pillar stability testing framework (benchmarks, chaos, soak, properties, security, contract)
- 31 new tests (4 Criterion, 9 chaos, 4 soak, 9 property + memory profiling)
- Automated weekly CI workflow with baseline extraction and regression detection
- Experiment tracking system (EXP-001 through EXP-005) with evidence collection
- Framework documentation and next-steps roadmap
- **Decision**: Valuable infrastructure work; committing to Unreleased, will ship with v3.4

## Completed: v3.2 — Type Duplication Cleanup
Completed: 2026-04-12

- [x] Unify canonical chat/provider types across crates (REF-36)
- [x] Delete deprecated LlmBackend trait from core/backend.rs
- [x] Delete dead converge_pack::Invariant fork
- [x] Delete dead ExperienceAppender/ExperienceReplayer from core/traits/store.rs
- [x] Unify FinishReason within converge-llm
- [x] Rename kernel_boundary::TraceLink to ReplayTrace
- [x] Consolidate IntentId to single definition in core
- [x] Replace llm/prompt_dsl.rs with re-exports from core (354 -> 10 lines)

## Completed: v3.1 — Documentation & Contract Hardening
Completed: 2026-04-12

- [x] Fix dead `docs/` links in README.md and SECURITY.md (8 broken refs)
- [x] Update SECURITY.md supported versions to 3.0.x
- [x] Update CHANGELOG.md through v3.0.2
- [x] Update CLAUDE.md dep graph, version, rust-version to match reality
- [x] Update DEVELOPMENT.md workspace layout and publish order
- [x] Update README.md crate table to 6 canonical crates
- [x] Add missing examples to examples/README.md (4 unlisted)
- [x] Fix proto copyright headers (Aprio One AB -> Reflective Labs, MIT license)
- [x] Add doc comments to converge-client public API
- [x] Add `//!` crate doc to converge-analytics

## Completed: v3.0 — Contract Stabilization
Completed: 2026-04-11

- [x] ADR-001: Canonical public crates (pack, provider-api, model, kernel, protocol, client)
- [x] ADR-002: Single truth pipeline
- [x] ADR-003: Pack authoring contract
- [x] ADR-004: Contract stabilization and freeze
- [x] Extract auth, ledger, nats, consensus, observability crate boundaries
- [x] 26 proof tests, regression gate frozen
- [x] Knowledge lifecycle moved to organism-domain
