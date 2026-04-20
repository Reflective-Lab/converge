---
tags: [architecture]
source: mixed
---
# System Overview

Converge is a **pure Rust Suggestor OS** for building correctness-first,
context-driven, multi-suggestor systems that provably converge.

The core contract is async-capable but runtime-agnostic: async trait boundaries are allowed, while runtimes, task spawning, and I/O stay at the edges.

## Canonical Contracts

The canonical external API boundaries are defined in:

- [[Architecture/API Surfaces]]
- [[Architecture/ADRs/ADR-001-canonical-public-crates]]
- [[Architecture/ADRs/ADR-002-truth-pipeline]]
- [[Architecture/ADRs/ADR-003-pack-authoring-contract]]
- [[Architecture/ADRs/ADR-004-contract-stabilization]]

If older sections of this file conflict with those documents, the API surfaces
page and ADRs win.

## Core Axioms

1. Context is the only shared state
2. Suggestors never call each other
3. Execution is graph-based, not linear
4. Context evolution is monotonic
5. Convergence is explicit and observable
6. Correctness is verified semantically
7. LLMs may suggest, never decide

## System View

```
┌──────────────────────────────────────────┐
│ Specification Layer (Gherkin)            │
│ Behavioral invariants & success criteria │
└──────────────────────────────────────────┘
┌──────────────────────────────────────────┐
│ Orchestration Layer                      │
│ Execution graph & convergence engine     │
└──────────────────────────────────────────┘
┌──────────────────────────────────────────┐
│ Suggestor Layer                              │
│ Deterministic · LLM · Solver · IO        │
└──────────────────────────────────────────┘
┌──────────────────────────────────────────┐
│ Context Layer                            │
│ Typed, shared, monotonic job state       │
└──────────────────────────────────────────┘
┌──────────────────────────────────────────┐
│ Tool Layer                               │
│ LLMs · Search · Solvers · APIs · MCP     │
└──────────────────────────────────────────┘
```

## Workspace Layout

```
crates/
  model/          Curated semantic model surface
  pack/           Canonical pack authoring contract
  protocol/       Canonical generated wire contract (`converge.v1`)
  traits/         Deprecated compatibility facade
  kernel/         Curated in-process embedding API
  client/         Canonical remote Rust SDK
  core/           Kernel implementation and truth pipeline enforcement
  provider-api/   Canonical provider capability contract
  provider/       LLM backends (Anthropic, OpenAI, Gemini, Ollama, …)
  domain/         Domain suggestors, packs, and governed flow actions
  experience/     Event-sourced audit ledger
  knowledge/      Vector search knowledge base
  mcp/            Model Context Protocol (client + server)
  optimization/   Constraint solving (optional OR-Tools FFI)
  auth/           Authentication, authorization, cryptography (publish = false)
  consensus/      Raft consensus adapter (publish = false)
  ledger/         Append-only context ledger (publish = false)
  nats/           NATS messaging adapter (publish = false)
  observability/  Audit, telemetry, and metrics (publish = false)
  analytics/      Polars + Burn analytics (publish = false)
  llm/            Local inference kernel (publish = false)
  policy/         Cedar policy engine and default flow gate authorizer (publish = false)
  runtime/        HTTP/gRPC servers (publish = false)
  remote/         Compatibility CLI on top of client + protocol
  tool/           Dev tools, Gherkin validator (`cz` binary)
  application/    Single `converge` binary (publish = false)

schema/
  proto/          Protocol Buffer definitions
  openapi/        OpenAPI specifications

examples/         Standalone example crates
kb/Architecture/  Canonical architecture docs and ADRs
```

## Dependency Graph (leaf → root)

```
converge-pack            (no internal deps)
converge-provider-api    (no internal deps)
converge-protocol        (no internal deps)
converge-traits          -> pack, provider-api (compatibility only)
converge-core            -> pack
converge-model           -> core, pack
converge-kernel          -> core, pack
converge-client          -> protocol
converge-mcp             (no internal deps)
converge-provider        → core, pack, provider-api
converge-domain          → core, policy, provider
converge-experience      → core
converge-knowledge       → mcp (server feature)
ortools-sys              (no deps, FFI)
converge-optimization    → ortools-sys (optional)
converge-auth            (no internal deps)
converge-consensus       (no internal deps)
converge-ledger          (no internal deps)
converge-nats            (no internal deps)
converge-observability   (no internal deps)
converge-analytics       → core, domain, provider
converge-llm             → core, domain, provider
converge-policy          → core
converge-axiom            → core, provider
converge-remote          → client, protocol
converge-runtime         → core, provider, protocol, tool
converge-application     → core, provider, domain, tool, mcp, knowledge, …
```

## Execution Model

This section describes the enforced implementation shape.

ADR-002 defines the truth pipeline enforced in code.

### Basic convergence loop
```
initialize context from RootIntent

repeat
  determine eligible suggestors (pure, side-effect free)
  filter by active packs        (only suggestors in activated packs run)
  execute eligible suggestors   (read-only context via Context; scheduling is runtime-defined)
  collect AgentEffects          (buffered proposals only)
  promote proposals → facts     (promotion gate validates confidence, provenance)
  merge effects into context    (serialized, deterministic)
  evaluate criteria             (CriterionEvaluator checks success conditions)
until convergence or termination
```

### Application-level truth execution
```
Application builds TypesRootIntent from TruthDefinition
  → intent carries: active_packs, success_criteria, budget, constraints

Application creates Engine, registers suggestors in packs
  → engine.register_suggestor_in_pack("compliance-pack", screener_agent)

Application calls run_with_types_intent_and_hooks()
  → engine runs convergence loop
  → CriterionEvaluator checks each criterion after convergence
  → ExperienceEventObserver captures events during the run

Engine returns ConvergeResult
  → context: final state with all facts
  → criteria_outcomes: per-criterion Met/Unmet/Blocked/Indeterminate
  → stop_reason: Converged | CriteriaMet | HumanInterventionRequired | BudgetExhausted

Application projects ConvergeResult into domain state
  → reads facts from context, writes to its own storage
```

**Convergence**: `Contextₙ₊₁ == Contextₙ` — no new facts, no new intents, no state change.

**Termination**: convergence reached, criteria met, budgets exhausted, invariants violated, or human intervention required.

**Guarantees**: determinism, termination (budgets), isolation (suggestors can't affect each other), auditability (full provenance on every fact and proposal).

## Feature Gates

The workspace uses Cargo features so consumers only pull what they need.

**Individual crates** (supported library consumers):
```toml
converge-kernel = "3"     # embed the engine in-process
converge-pack = "3"       # author suggestors and invariants
converge-client = "3"     # connect to a remote runtime
```

**Umbrella binary** (`converge-application`):
| Feature        | What it enables                        | Heavy deps        |
|----------------|----------------------------------------|-------------------|
| `tui`          | Interactive terminal UI (default)      | ratatui, crossterm|
| `knowledge`    | Knowledge base commands                | —                 |
| `mcp`          | MCP server (`converge mcp serve`)      | axum, tower       |
| `llm`          | Local inference kernel                 | burn, llama-burn  |
| `analytics`    | Analytics pipeline                     | polars, burn      |
| `optimization` | Constraint solving                     | ortools-sys (FFI) |
| `full`         | Everything                             | all of the above  |

## API Surface

The supported external Rust contracts are:

- `converge-pack`
- `converge-provider-api`
- `converge-model`
- `converge-kernel`
- `converge-protocol`
- `converge-client`

The current external network contract is:

- protobuf package `converge.v1` in `schema/proto/converge.proto`

`converge-traits` is deprecated compatibility only.
`converge-core` is implementation, not the intended external contract.
`converge-remote` is a compatibility CLI, not the canonical Rust SDK.

### Engine
```rust
let mut engine = Engine::new();
engine.register_suggestor(suggestor);                      // global suggestor
engine.register_suggestor_in_pack("pack-id", suggestor);   // pack-scoped suggestor
engine.run(context).await;                      // basic convergence
engine.run_with_types_intent_and_hooks(          // application-level truth execution
    context, &intent, TypesRunHooks {
        criterion_evaluator: Some(evaluator),
        event_observer: Some(observer),
    },
).await;
```

### Context
```rust
let context = ContextState::new();
context.has(ContextKey::Seeds);
context.get(ContextKey::Seeds);          // → iterator of &Fact
context.get(ContextKey::Evaluations);
context.get(ContextKey::Diagnostic);
context.add_input(ContextKey::Seeds, "seed-1", "initial observation");
context.add_input_with_provenance(ContextKey::Seeds, "seed-2", "source data", "operator");
```

### Suggestor trait
```rust
#[async_trait::async_trait]
trait Suggestor {
    fn name(&self) -> &str;
    fn dependencies(&self) -> &[ContextKey];
    fn accepts(&self, ctx: &dyn Context) -> bool;
    async fn execute(&self, ctx: &dyn Context) -> AgentEffect;
}
```

Suggestors never call other suggestors. Suggestors never mutate context directly. Suggestors only emit effects.

### AgentEffect
```rust
// A suggestor can emit draft proposals only.
AgentEffect {
    proposals: vec![...],   // need promotion gate validation
}
AgentEffect::with_proposal(proposed_fact)  // convenience
AgentEffect::with_proposals(vec![...])     // convenience
AgentEffect::empty()                       // nothing to contribute
```

### ProposedFact
```rust
ProposedFact {
    key: ContextKey::Evaluations,
    id: "compliance:screen:acme".into(),
    content: payload_json,
    confidence: 0.85,           // how confident the suggestor is (0.0–1.0)
    provenance: "suggestor:screener".into(),  // who proposed it
}
```

### Truth Execution
```rust
// Application declares truths
trait TruthCatalog {
    fn list_truths(&self) -> Vec<TruthDefinition>;
    fn find_truth(&self, key: &str) -> Option<TruthDefinition>;
}

// Application evaluates criteria
trait CriterionEvaluator {
    fn evaluate(&self, criterion: &Criterion, context: &Context) -> CriterionResult;
}

// Four-way typed result
enum CriterionResult {
    Met { evidence: Vec<FactId> },
    Blocked { reason: String, approval_ref: Option<String> },
    Unmet { reason: String },
    Indeterminate,
}

// Durable context across runs
trait ContextStore {
    fn load_context(&self, scope_id: &str) -> impl Future<Output = Result<Option<Context>>>;
    fn save_context(&self, scope_id: &str, context: &Context) -> impl Future<Output = Result<()>>;
}
```

### ConvergeResult
```rust
ConvergeResult {
    context: Context,              // final state
    cycles: u32,                   // how many cycles ran
    converged: bool,               // did it reach a fixed point?
    stop_reason: StopReason,       // why it stopped
    criteria_outcomes: Vec<CriterionOutcome>,  // per-criterion results
}

enum StopReason {
    Converged,
    CriteriaMet { .. },
    HumanInterventionRequired { criteria, approval_refs },
    CycleBudgetExhausted { .. },
    FactBudgetExhausted { .. },
    // ...
}
```

## Schema

Protocol definitions live in `schema/`:
- `schema/proto/converge.proto` — Main gRPC API (bidirectional streaming for mobile/CLI)
- `schema/proto/knowledge.proto` — Knowledge base service
- `schema/proto/kernel.proto` — LLM reasoning kernel (GPU-isolated)

## One-sentence takeaway

> Converge executes suggestors against a read-only context and commits knowledge serially to guarantee convergence.

See also: [[Architecture/Crate Map]], [[Architecture/Engine Execution Model]], [[Architecture/Hexagonal Architecture]]
