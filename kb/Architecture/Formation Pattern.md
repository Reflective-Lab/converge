---
tags: [architecture, formations]
source: mixed
---
# Formation Pattern

How upper layers (Organism, Helms, applications) assemble teams of
heterogeneous agents and run them in Converge.

## What is a Formation?

A formation is:
- A team of Suggestors (from any crate/domain)
- A seed Context (initial facts)
- An execution Budget

It's a hypothesis: "this team, with these seeds, will converge on a
good answer to this intent."

## Building a Formation

```rust
use converge_kernel::{Budget, ContextKey, ContextState, Engine, ProposedFact};
use converge_optimization::SolverSuggestor;
use converge_policy::PolicyGateSuggestor;
use organism_simulation::OutcomeSimulationAgent;

// 1. Create engine
let mut engine = Engine::with_budget(Budget::new(20));

// 2. Register heterogeneous agents
engine.register_suggestor(SolverSuggestor::budget_allocation(constraints));
engine.register_suggestor(PolicyGateSuggestor::new(policy));
engine.register_suggestor(OutcomeSimulationAgent::default_config());
engine.register_suggestor(my_llm_planner);

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
- Policy gates from `converge-policy`
- Analytics agents from `converge-analytics`
- Domain pack agents from `converge-domain`
- Custom LLM agents from `organism-planning`
- Simulation agents from `organism-simulation`
- Application-specific agents from `helms`

They all converge together in one Engine run.

## Organism's Role

Organism is the formation guru — it decides WHICH agents to include:

1. Analyze the intent (complexity, domain, constraints)
2. Select appropriate agent types (LLM for reasoning, solver for scheduling, etc.)
3. Assemble the formation
4. Run it (possibly multiple competing formations)
5. Evaluate results, reform if needed
6. Feed outcomes into learning

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

Analytics packs (`converge-analytics`) are designed to compose with optimization
and policy packs in formations. Common patterns:

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

See also: [[Suggestor Contract]], [[Hexagonal Architecture]], [[Analytics Packs]]
