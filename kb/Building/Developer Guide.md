---
tags: [building, guide]
source: mixed
version: 3.9.1
date: 2026-05-15
---
# Developer Guide for Converge 3.9.1

This guide is the practical entry point for developers building against
Converge 3.9.1 or changing the Converge foundation itself.

Converge is a correctness-first, context-driven multi-suggestor runtime. The
kernel owns convergence, proposal promotion, invariants, HITL pauses, and run
integrity. It does not own product orchestration, formation strategy, source
connectors, vendor SDK wiring, or application deployment.

## Version Baseline

Converge 3.9.1 uses:

| Item | Value |
|---|---|
| Rust edition | 2024 |
| MSRV | 1.96.0 |
| License | MIT |
| Public API promise | six canonical public crates |
| Core rule | suggestors propose, the engine promotes |

Read [[Philosophy/Nine Axioms]] before changing behavior. The short version:
authority is typed, context is the coordination channel, facts are append-only,
and `Fact` construction is engine-owned.

## Choose the Narrowest Crate

Depend on the smallest public surface that fits the job.

| If you are... | Start with... | Why |
|---|---|---|
| writing reusable suggestors, packs, or invariants | `converge-pack` | authoring contract |
| embedding the engine in a Rust process | `converge-kernel` | runnable in-process API |
| sharing governed semantic types | `converge-model` | curated semantic surface |
| writing provider or tool adapters | `converge-provider` | backend identity and capability routing |
| calling a remote runtime from Rust | `converge-client` | typed Rust SDK |
| integrating over gRPC/protobuf | `converge-protocol` | generated `converge.v1` wire types |

Do not treat `converge-core` as the default downstream dependency. It contains
engine implementation details. Some internal crates are publishable today, but
the compatibility promise is the six-crate public surface.

Public placement rule:

- semantics in `converge-model`
- authoring in `converge-pack`
- runnable machinery in `converge-kernel`

See [[Architecture/API Surfaces]] and [[Architecture/Crate Map]].

## Add Converge to an Application

For an embedded application:

```toml
[dependencies]
converge-kernel = "3.9.1"
converge-model = "3.9.1"
async-trait = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

For a reusable pack:

```toml
[dependencies]
converge-pack = "3.9.1"
```

For provider or tool adapters:

```toml
[dependencies]
converge-provider = "3.9.1"
```

For a remote Rust client:

```toml
[dependencies]
converge-client = "3.9.1"
```

## First Embedded Run

```rust
use converge_kernel::{
    AgentEffect, Context, ContextKey, ContextState, Engine, Provenance, ProvenanceSource, Suggestor,
    TextPayload,
};

struct SeedSuggestor;

#[derive(Clone, Copy)]
struct SeedProvenance;

impl ProvenanceSource for SeedProvenance {
    fn as_str(&self) -> &'static str {
        "suggestor:seed"
    }
}

const SEED_PROVENANCE: SeedProvenance = SeedProvenance;

#[async_trait::async_trait]
impl Suggestor for SeedSuggestor {
    fn name(&self) -> &str {
        "seed"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        !ctx.has(ContextKey::Seeds)
    }

    fn provenance(&self) -> Provenance {
        SEED_PROVENANCE.provenance()
    }

    async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
        AgentEffect::with_proposal(
            SEED_PROVENANCE.proposed_fact(
                ContextKey::Seeds,
                "seed:observation-1",
                TextPayload::new("Monthly active users grew 15%"),
            )
            .with_confidence(0.95),
        )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor);

    let result = engine.run(ContextState::new()).await?;

    assert!(result.converged);
    assert!(result.context.has(ContextKey::Seeds));
    Ok(())
}
```

`converge-kernel` re-exports the common authoring types so embedded
applications do not need to import `converge-pack` directly unless they are
publishing pack crates.

## The Suggestor Contract

Every participant in the loop implements `Suggestor`:

```rust
#[async_trait::async_trait]
pub trait Suggestor: Send + Sync {
    fn name(&self) -> &str;
    fn dependencies(&self) -> &[ContextKey];
    fn accepts(&self, ctx: &dyn Context) -> bool;
    async fn execute(&self, ctx: &dyn Context) -> AgentEffect;
}
```

The rules are strict:

- `name()` is for logs and provenance, not ordering.
- `dependencies()` declares the context keys that can wake the suggestor.
- `accepts()` is pure, deterministic, and side-effect free.
- `execute()` reads context and returns an `AgentEffect`.
- Suggestors do not mutate context and do not call each other.
- Idempotency is checked through context, not internal `has_run` flags.

Use dependency keys to model sequencing:

```text
Seeds -> Evaluations -> Strategies
```

The engine controls eligibility, promotion, merge order, and convergence. See
[[Architecture/Suggestor Contract]] and [[Building/Writing Agents]].

## Context and Facts

Context is the only shared state. Suggestors see `&dyn Context`:

```rust
ctx.has(ContextKey::Seeds);
ctx.get(ContextKey::Evaluations);
ctx.count(ContextKey::Strategies);
```

Facts are partitioned by `ContextKey`. Common keys include:

| Key | Use |
|---|---|
| `Seeds` | external input and initial evidence |
| `Hypotheses` | tentative conclusions |
| `Strategies` | action plans and recommendations |
| `Constraints` | limits and boundary conditions |
| `Signals` | observations and environmental data |
| `Competitors` | competitive intelligence |
| `Evaluations` | scores, rankings, and decisions |
| `Proposals` | LLM-generated suggestions awaiting validation |
| `Diagnostic` | debug or malformed-input information |
| `Votes`, `Disagreements`, `ConsensusOutcomes` | governance state |

Normal consumers construct `ProposedFact`, not `Fact`:

```rust
use converge_pack::{ContextKey, FactPayload, ProvenanceSource};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VendorEvaluation {
    score: f64,
}

impl FactPayload for VendorEvaluation {
    const FAMILY: &'static str = "example.vendor_evaluation";
    const VERSION: u16 = 1;
}

#[derive(Clone, Copy)]
struct ExampleProvenance;

impl ProvenanceSource for ExampleProvenance {
    fn as_str(&self) -> &'static str {
        "suggestor:vendor-evaluator"
    }
}

const EXAMPLE_PROVENANCE: ExampleProvenance = ExampleProvenance;

let proposal = EXAMPLE_PROVENANCE.proposed_fact(
    ContextKey::Evaluations,
    "evaluation:vendor-1",
    VendorEvaluation { score: 0.82 },
)
.with_confidence(0.82);
```

Serialized materialization happens at borders through the wire shape:

```rust
let wire = proposal.to_wire()?;
let mut registry = converge_pack::PayloadRegistry::new();
registry.register::<VendorEvaluation>();
let restored = converge_pack::ProposedFact::from_wire(wire, &registry)?;
```

Confidence is method-based and clamped by the type:

```rust
use converge_pack::{CONFIDENCE_STEP_MAJOR, CONFIDENCE_STEP_MINOR, TextPayload};

let proposal = EXAMPLE_PROVENANCE.proposed_fact(
    ContextKey::Hypotheses,
    "h:1",
    TextPayload::new("supported"),
)
.with_confidence(0.5)
.adjust_confidence(CONFIDENCE_STEP_MAJOR)
.adjust_confidence(CONFIDENCE_STEP_MINOR);
```

Add `converge-pack` directly when you want the authoring-only confidence step
constants in an embedded application.

Do not reach for a public `Fact::new`. Authoritative fact construction is not a
consumer API. The engine promotes proposals into context facts.

See [[Building/Context Keys]] and [[Architecture/Type Protocol]].

## Seeding External Input

Embedders stage input as proposals:

```rust
let mut context = ContextState::new();
context.add_input_with_provenance(
    ContextKey::Seeds,
    "root-intent",
    serde_json::json!({ "company": "Acme Corp" }).to_string(),
    "organism-runtime",
)?;
```

For actor-aware admission, use `converge_kernel::admission`:

```rust
use converge_kernel::{
    ContextKey, ContextState,
    admission::{
        AdmissionActor, AdmissionActorKind, AdmissionContent, AdmissionRequest, AdmissionSource,
        admit_observation,
    },
};

let mut context = ContextState::new();
let request = AdmissionRequest::new(
    AdmissionActor::new("organism-runtime", AdmissionActorKind::System)?,
    AdmissionSource::new("truth-document")?,
    ContextKey::Seeds,
    "truth-doc-1",
    AdmissionContent::new(r#"{"claim":"approved source"}"#)?,
)?;

let receipt = admit_observation(&mut context, request)?;
assert!(receipt.staged());
```

External input is still governed. Staging an observation is not authority to
create a fact.

## Running the Engine

Basic execution:

```rust
let mut engine = Engine::new();
engine.register_suggestor(MySuggestor);
engine.register_suggestor_in_pack("analysis-pack", AnotherSuggestor);
engine.register_invariant(MyInvariant);

let result = engine.run(context).await?;
```

For human-in-the-loop gates:

```rust
engine.set_hitl_policy(policy);

match engine.run_with_hitl(context).await {
    RunResult::Complete(result) => {
        let result = result?;
        assert!(result.converged);
    }
    RunResult::Paused(pause) => {
        // collect a typed GateDecision from the human authority, then:
        let resumed = engine.resume(pause, decision).await;
    }
}
```

Use `run()` when the host wants a completed `ConvergeResult` or an error. Use
`run_with_hitl()` when a governed pause is a valid outcome.

## Provider and Tool Integration

`converge-provider` is the provider contract. It owns:

- `Backend` and `BackendKind`
- `Capability`
- `ChatBackend`, `DynChatBackend`, `ChatRequest`, and `ChatResponse`
- `ResponseFormat`
- `BackendRequirements`, `ProviderRequest`, and `ProviderAssignment`
- `ChatBackendRegistry` and typed registry values
- `ChatBackendSelectionConfig` and `SelectionCriteria`

Concrete vendor implementations do not live in this foundation workspace in
3.9.1. Generic adapters such as LLM chat, search, fetch, feed, embeddings,
OpenAPI tools, and GraphQL tools live in Manifold. Source-specific connector
ports live in Embassy. Policy implementations live in Arbiter. Knowledge lives
in Mnemos. Analytics lives in Prism.

The host assembles already-constructed backend handles, then passes them across
Converge's contract boundary. Converge selects and routes; it does not inspect
secrets or instantiate vendor SDKs.

See [[Architecture/Extension Topology]].

## Formation Support

For embedders, formation APIs are grouped under `converge_kernel::formation`:

```rust
use converge_kernel::formation::{
    FormationCatalog, FormationRequest, FormationTemplate, FormationTemplateMetadata,
    ProfileSnapshot, ProviderRequest, ProviderSelectionSuggestor, SuggestorCapability,
    SuggestorRole,
};
```

The stable boundary begins at structured requests:

- `FormationTemplateQuery`
- `FormationCatalog`
- `FormationRequest`
- `ProviderRequest`

If intent already arrives structured, seed those requests directly. If intent
arrives as loose user text, compile it upstream into structured requests before
Converge formation machinery runs.

Converge does not automatically introspect registered suggestors. The embedder
or upper layer builds the `ProfileSnapshot` catalog when it wires profiled
suggestors into a run.

See [[Architecture/Embedding Quick Start]].

## Remote Runtime Integration

Use `converge-client` from Rust:

```rust
use converge_client::{ConvergeClient, v1};

let mut client = ConvergeClient::connect("http://127.0.0.1:50051").await?;
let capabilities = client
    .get_capabilities(v1::GetCapabilitiesRequest {
        device_id: "cli-1".to_string(),
        app_version: "3.9.1".to_string(),
        platform: "cli".to_string(),
    })
    .await?;
```

Use `converge-protocol` when you need the generated `converge.v1` protobuf and
gRPC types directly. Non-Rust consumers should integrate against
`schema/proto/converge.proto`.

The runtime crate is an internal service shell. Do not import it just to share
contract types.

## Extension Placement

Converge keeps universal contracts. Implementation-heavy or SDK-facing code
lives outside the foundation.

| Work | Home | Floor against Converge 3.9.1 |
|---|---|---|
| generic LLM, search, fetch, feed, embedding, and tool adapters | Manifold | `converge-manifold-adapters = "1.1.1"` |
| source-specific connector ports and adapters | Embassy | `converge-embassy-* = "1.1.1"` |
| knowledge, recall, retrieval, vector memory | Mnemos | `converge-mnemos-knowledge = "1.2.1"` |
| analytics, feature pipelines, monitoring | Prism | `converge-prism-analytics = "2.0.0"` |
| trained model packs and RF/SVM/ANFIS suggestors | Crucible | `converge-crucible-models = "0.2.1"` |
| Cedar policy engines and policy suggestors | Arbiter | `converge-arbiter-policy = "2.0.0"` |
| native OR-Tools and HiGHS solver integrations | Ferrox | `converge-ferrox-solver = "0.7.0"` |
| SMT-backed safety and assurance suggestors | Soter | `converge-soter-smt = "0.2.1"` |
| domain packs and worked exemplars | Atelier | `atelier-domain = "1.0.0"` |
| deployment assembly, secrets, processes, Docker, databases | Runtime Runway or product repos | — |

The dependency direction is:

```text
converge contracts <- extensions <- products / deployments
```

Foundation must not import extension repositories.

## Repository Development

Start every repo session with:

```bash
just focus
```

Common commands:

| Command | Purpose |
|---|---|
| `just build` | release build |
| `just build-quick` | quick release profile |
| `just check` | workspace check |
| `just test` | default test suite |
| `just test-all` | all workspace targets |
| `just test-layout` | Rust test file placement guard |
| `just lint` | format and clippy gate |
| `just sec-gate` | security regression gate |
| `just sec-deny` | dependency audit |
| `just doc` | workspace docs |
| `just git-hygiene` | branch, worktree, and release hygiene |
| `just size-audit` | runtime and kernel packaging baseline |

`just lint` must be clean before work is considered done.

## Code Rules

These rules are part of the platform contract:

- no `unsafe` code
- use typed enums instead of strings with semantics
- use newtypes for semantic identifiers
- use validated types for closed numeric domains
- use `serde(deny_unknown_fields)` for known config schemas unless openness is deliberate
- keep `converge-core` pure: no I/O, persistence, hidden background work, or non-determinism
- add tests at public boundaries, wire formats, feature gates, and algorithmic invariants
- prefer stronger types over negative tests for internal misuse
- put Rust tests under `src/` or `tests/`, with source-tree tests explicitly linked
- do not add feature flags or compatibility shims for new behavior
- do not inline dependency versions in crate manifests; use workspace dependencies
- keep the root checkout clean and use a topic branch or dedicated worktree for non-trivial changes

## 3.9.0 Migration Notes

Use the current contract shape:

| Changed or stale name | Current API |
|---|---|
| raw semantic strings in `ProposedFact::new(...)` | typed `FactPayload` values, usually through `*_PROVENANCE.proposed_fact(...)` |
| ad hoc provenance strings inside fact-emitting suggestors | `ProvenanceSource` marker + `Suggestor::provenance()` override |
| empty provenance on a fact-emitting suggestor | `ConvergeError::EmptyProvenance` from the engine |
| `gate::ProvenanceEnvelope` | `gate::AuditEnvelope` (`ProvenanceEnvelope` is a deprecated compatibility alias) |
| local `FormationKind` definitions | `converge-pack::FormationKind`, re-exported from `converge-core` |
| string-built runtime config | `ExecutionIdentity::runtime_config_from_typed(...)` or `with_runtime_config_typed(...)` |

`Context::formation_kind()` now reports an optional formation kind and defaults
to `None` for contexts that do not carry formation metadata.

## 3.8.1 Migration Notes

Use the current names:

| Removed or stale name | Current API |
|---|---|
| `Agent` | `Suggestor` |
| `ContextView` | `Context` |
| `Context::new()` | `ContextState::new()` |
| `register_in_pack(...)` | `register_suggestor_in_pack(...)` |
| public `Fact::new(...)` | unavailable to normal consumers |
| `kernel-authority` feature | removed |
| `converge-provider-adapters` | removed; adapter implementations live in Manifold |

Provider/tool contracts own clean domain names. Concrete implementations use
adapter-qualified names such as `OpenAiChatAdapter`, `BraveSearchAdapter`, and
`OpenApiToolAdapter`.

## Before Opening a PR

1. Confirm the change belongs in the foundation, not an extension.
2. Check the public surface: `converge-pack`, `converge-provider`,
   `converge-model`, `converge-kernel`, `converge-protocol`, or
   `converge-client`.
3. Update KB pages before or alongside architecture changes.
4. Run the narrowest useful test first.
5. Run `just lint`.
6. Run broader checks when the change touches shared behavior, public contracts,
   security, runtime wiring, or release gates.

Further reading:

- [[Architecture/Core Ideas]]
- [[Architecture/API Surfaces]]
- [[Architecture/Embedding Quick Start]]
- [[Architecture/Suggestor Contract]]
- [[Architecture/Type Protocol]]
- [[Architecture/Extension Topology]]
- [[Workflow/Git Strategy]]
