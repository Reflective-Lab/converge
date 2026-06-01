---
tags: [architecture, formations]
source: mixed
---
# Formation Pattern

How embedders and upper layers assemble heterogeneous teams and run them in
Converge without depending on internal crates.

## Public Surface Rule

This is the formation mantra:

- semantics in `converge-model`
- authoring in `converge-pack`
- runnable machinery in `converge-kernel`

For embedders, the grouped offering API is `converge_kernel::formation`.

## What is a Formation?

A formation is:
- A team of Suggestors (from any crate/domain)
- A seed Context (initial facts)
- An execution Budget

It's a hypothesis: "this team, with these seeds, will converge on a
good answer to this intent."

## Building a Formation

```rust
use converge_kernel::{
    formation::{FormationAssemblySuggestor, ProviderSelectionSuggestor},
    Budget, ContextKey, ContextState, Engine, ProposedFact,
};

// 1. Create engine
let mut engine = Engine::with_budget(Budget::new(20));

// 2. Register heterogeneous agents
engine.register_suggestor(optimizer);
engine.register_suggestor(policy_gate);
engine.register_suggestor(simulation_agent);
engine.register_suggestor(my_llm_planner);
engine.register_suggestor(FormationAssemblySuggestor::new(catalog));
engine.register_suggestor(ProviderSelectionSuggestor::new(backends));

// 3. Seed context
let mut ctx = ContextState::new();
ctx.add_proposal(ProposedFact::new(
    ContextKey::Seeds, "intent", intent_json, "organism"
))?;

// 4. Run convergence
let result = engine.run(ctx).await?;
```

## Can Formations Mix Suggestors From Different Crates?

**Yes.** The Suggestor trait is defined in `converge-pack`. Any crate that
implements it can participate. A single formation can include:

- Optimization solvers from `converge-optimization`
- Policy gates from the **arbiter** extension (formerly `converge-policy`)
- Analytics agents from the **prism** extension (formerly `converge-analytics`)
- Knowledge / recall agents from the **mnemos** extension (formerly `converge-knowledge`)
- Domain pack agents from the **atelier** showcase (formerly `converge-domain`)
- Custom LLM agents from `organism-planning`
- Simulation agents from `organism-simulation`
- Application-specific agents from `helms`

They all converge together in one Engine run.

## Structured vs Loose Intent

The canonical formation contract begins at structured requests:

- `FormationTemplateQuery`
- `FormationCatalog`
- `FormationRequest`
- `ProviderRequest`

Two upstream patterns are valid:

- structured intent
  - a seeder writes the requests directly
- loose intent
  - an upstream suggestor matches a `FormationTemplate` from the catalog and
    then compiles it into the requests

The built-in formation machinery does not care which upstream path produced the
requests. It starts at the structured handoff.

## Formation Templates vs Suggestor Catalogs

There are two distinct catalogs in the current design:

| Layer | Type | Purpose |
|---|---|---|
| Upper-layer selection | `FormationCatalog` | Match problem signals (`keywords`, `entities`, required capabilities) to reusable formation templates |
| Concrete assembly | `Vec<ProfileSnapshot>` | Match required roles to the registered suggestors available in this run |

`FormationTemplate::to_request(...)` intentionally carries only required roles
today. Template-level capability metadata stays on the catalog entry because the
current `FormationRequest.required_capabilities` field is still interpreted as a
global per-suggestor eligibility gate by `FormationAssemblySuggestor`.

## Organism's Role

Organism is the formation guru — it decides WHICH agents to include:

1. Analyze the intent (complexity, domain, constraints)
2. Select appropriate agent types (LLM for reasoning, solver for scheduling, etc.)
3. Assemble the formation
4. Run it (possibly multiple competing formations)
5. Evaluate results, reform if needed
6. Feed outcomes into learning

See [[Formation Building Review]] for the current Organism-layer guidance on
formation compilers, tournaments, descriptor contracts, HITL graduation, and
OpenClaw guard rails.

## Competing Formations

Multiple formations can run concurrently on the same intent:
- Formation A: heavy LLM reasoning + light policy
- Formation B: optimization solver + strict policy + adversarial
- Compare results, pick the winner
- Learn which formation shapes work for which intent types

## What Goes IN a Formation vs What Stays Outside

| Inside the formation (Suggestors) | Outside (Organism runtime) |
|---|---|
| Planning agents | Intent admission (quick structural check) |
| Adversarial agents | Formation assembly (which agents?) |
| Simulation agents | Result evaluation (which formation won?) |
| Policy gates | Learning (update priors for next time) |
| Optimization solvers | |
| Analytics agents | |
| Domain logic | |

## ContextKey Flow Within a Formation

```
Seeds → Hypotheses → Strategies → Evaluations → Constraints
  ↑                                                   ↓
  └───── agents read/write to different keys ─────────┘
```

Agents self-organize via dependencies:
- Planner depends on Seeds, writes Strategies
- Skeptic depends on Strategies, writes Constraints
- Simulator depends on Strategies, writes Evaluations
- Policy gate depends on Strategies, writes Constraints (if violated)

The Engine's fixed-point detection handles the coordination.

## Analytics Pack Compositions

Analytics packs (the **prism** extension, `~/dev/reflective/mosaic-extensions/prism`, formerly
`converge-analytics`) are designed to compose with optimization and policy
packs in formations. Common patterns:

| Formation | Packs composed | Use case |
|---|---|---|
| Detect → Triage → Scale | anomaly_detection → anomaly_triage → capacity_planning | SRE incident response |
| Segment → Route → Constrain | segmentation → lead_routing → PolicyGate | Sales operations |
| Forecast → Replenish → Budget | forecasting → inventory_replenishment → budget_allocation | Supply chain |
| Classify → Rank → Select | classification → ranking → vendor_shortlist | Procurement |
| Stats → Trend → Alert | descriptive_stats → trend_detection → anomaly_detection | Business intelligence |
| Similar → Segment → Price | similarity → segmentation → pricing_guardrails | Dynamic pricing |

Each pack reads from one ContextKey, writes to another. The Engine's fixed-point
detection coordinates the multi-cycle pipeline automatically.

See [[Analytics Packs]] for the full catalog, input schemas, and invariant definitions.

See also: [[Formation Building Review]], [[Suggestor Contract]], [[Hexagonal Architecture]], [[Analytics Packs]]
