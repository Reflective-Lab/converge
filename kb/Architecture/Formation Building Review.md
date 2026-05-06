---
tags: [architecture, formations, organism, review]
source: mixed
---
# Formation Building Review

Two-pass architecture review of how the Organism layer should compose
Formations from the Converge kernel's suggestor, provider, and governance
primitives. Conducted 2026-04-23 against the v3.7 codebase and KB.

## Assessment Summary

Converge is architecturally strong as a governed convergence kernel. The
universal Suggestor contract, append-only ExperienceStore, Cedar policy
integration, and deterministic fixed-point detection are the right
primitives. The gap is the meta-layer: the intelligence that discovers
capabilities, composes formations, runs tournaments, learns from priors,
and distills human decisions into policy.

The kernel explicitly delegates this to Organism (see [[Formation Pattern]]).

## Important Corrections

This review is grounded in the current v3.7 code and KB, not just the
aspirational contract.

- `ProfileSnapshot` already carries declarative write metadata via
  `output_keys`. The gap is not "no produces metadata exists"; the gap is that
  formation-participating suggestors and packs do not consistently expose or
  require that descriptor path yet.
- Do **not** rush a `produces()` method into the base `Suggestor` trait first.
  `Suggestor` is the universal authoring contract. Prefer a formation-specific
  descriptor trait or manifest for members that want to participate in
  formation planning.
- `FormationGenome` is a strong concept, but it should start as an
  **Organism-layer type**. Promote it into `converge-model` only if multiple
  upper layers genuinely need the same stable contract.
- Reuse the existing `FormationDecision` and `FormationOutcome` vocabulary
  rather than inventing a parallel outcome model. Extend those records with
  genome id, tournament id, lineage, and provider assignments as needed.

## Three Catalogs, Not One

The next layer should maintain three separate registries:

| Catalog | Owns | Evolves |
|---------|------|---------|
| **Formation templates** | Business archetypes (vendor selection, market entry, close-the-books, anomaly triage) | When new problem shapes emerge |
| **Suggestor descriptors** | What each suggestor/pack reads, writes, costs, and requires | When packs are added or updated |
| **Provider descriptors** | Backend selection under cost, latency, sovereignty, compliance, replay, and offline constraints | When backends change |

These evolve independently. A new search backend should not require
touching formation definitions.

## Capability Discovery Gap

Today suggestor discovery is manual. Examples build `ProfileSnapshot`s by
hand (`examples/live-formation/src/main.rs`, `examples/intent-codec-loop/
src/main.rs`), and `converge-pack` exposes no formation metadata.

The honest answer to "how does the next layer know what capabilities are
hiding inside packaged Suggestors?" is: **it does not**, unless you add a
required descriptor trait or manifest.

### Required: formation descriptor contract

Every pack/suggestor that wants formation assembly to reason about it must
declare:

```
SuggestorDescriptor {
    reads:             Vec<ContextKey>,     // dependencies()
    produces:          Vec<ContextKey>,     // output_keys() / manifest
    domain_tags:       Vec<DomainTag>,      // what domain it belongs to
    requires_backend:  bool,                // needs external I/O?
    cost_hint:         CostClass,           // already in ProfileSnapshot
    latency_hint:      LatencyClass,        // already in ProfileSnapshot
    suggestor_caps:    Vec<SuggestorCapability>, // already in ProfileSnapshot
    provider_needs:    Vec<BackendRequirements>, // richer than flat capability lists
    confidence_range:  (f64, f64),          // already in ProfileSnapshot
}
```

This should begin as a formation-specific trait or manifest, not as a change
to the universal `Suggestor` contract.

With reads and writes both declared, the Organism layer can automatically
compute valid topologies, minimal formations, and bottleneck analysis.

### Topology is ContextKey flow

The topology should be modeled as a `ContextKey` read/write graph plus
activation constraints, not as arbitrary agent-to-agent edges. Converge
coordination happens through shared context, not direct suggestor messaging.
Two formations with the same members but different `ContextKey` flow can
converge very differently.

## Complementary Team Assembly

The current `FormationAssemblySuggestor` (`crates/optimization/src/
suggestors/formation.rs`) filters for suggestors that each individually
match `required_capabilities`. That finds five Swiss Army knives. What you
need is a knife, a screwdriver, and a saw that together cover the
requirements.

The assembly algorithm needs to solve a **set cover** problem over
capabilities, not an individual filter. The Greedy Set Cover algorithm
already exists in `kb/Algorithms/Greedy Set Cover.md` as a planned
optimization pack — this is its first real customer.

`FormationTemplate::to_request()` (`crates/model/src/formation.rs`)
intentionally drops template capabilities today. The upper layer should not
overload the current `FormationRequest` for complementary assembly; it
should build its own compilation step.

## Per-Role Provider Routing

The provider API already has rich routing vocabulary (`crates/provider/
src/selection.rs`) including cost class, latency class, data sovereignty,
compliance, replay, offline, and modality. But the formation path only uses
`id + required_capabilities`.

In a serious formation — say, vendor evaluation for EU procurement — the
policy gate needs a replay-safe deterministic backend, the analyst needs a
fast cheap model for feature extraction, and the synthesizer needs the best
reasoning model available. One provider assignment per formation is wrong.
**Wire it per role.**

Do not bind Organism to the current internal capability registry as if it
were already the final public contract. Keep upper-layer routing centered on
`converge-provider` surfaces while the provider/capability contract is
still being realigned.

## FormationCompiler: The Highest-Leverage Addition

The meta-layer should be framed as a **compiler**, not a prompt:

```
Input:  Intent + three catalogs + ExperienceStore priors
Output: Executable FormationPlan

Passes:
  1. Intent classification → required capability set
  2. Experience recall → prior formation genomes for similar intents
  3. Set-cover assembly → complementary suggestor team
  4. Per-role provider routing → backend assignments
  5. Budget estimation → cycle/token/cost limits
  6. Policy preloading → relevant Cedar policies
  7. Topology validation → no cycles, all inputs satisfied
```

A compiler is deterministic, testable, and explainable. The learning/
tournament layer sits above the compiler — it chooses which compilation
strategy to try, not how to compile.

### The Guru as a Formation

The compiler itself can be implemented as a Formation (recursive, intentional):

- **IntentClassifier** — reads raw intent, writes typed Seeds
- **CapabilityMatcher** — reads Seeds, queries ProfileSnapshots, writes Hypotheses
- **ExperienceRecaller** — reads Seeds, queries ExperienceStore for prior formations, writes Hypotheses
- **TopologyPlanner** — reads Hypotheses, designs ContextKey flow graph, writes Strategies
- **BudgetEstimator** — reads Strategies + cost hints, writes Constraints
- **PolicyPreloader** — reads intent domain, loads Cedar policies, writes Constraints
- **FormationAssembler** — reads Strategies + Constraints, emits FormationPlan

Run this guru/planning formation as a **separate planning convergence**, not
inside the same context as the business formation it is compiling. That keeps
planning evidence, business evidence, and recall boundaries clean.

## Formation Tournament

Run → Measure → Compare → Update Priors.

For a given intent class:

1. Recall top-K formation genomes from ExperienceStore (by intent similarity)
2. If < K candidates, generate variants (add/remove roles, swap providers, widen budget)
3. Run candidates (parallel or sequential depending on cost tolerance)
4. Measure on **two separate axes**:
   - **Engine efficiency**: cycles to fixed point, fact/proposal ratio, constraint violations
   - **Business fitness**: outcome quality score, downstream real-world result
5. Update genome fitness scores; prune dominated genomes, keep Pareto front

Do not confuse time to fixed point with business success. A formation that
takes 12 cycles but produces the right vendor decision beats one that
converges in 3 cycles to the wrong answer. Score both axes, weight business
outcomes higher.

### Reuse existing formation vocabulary

Prefer extending the existing formation records over creating a new parallel
outcome taxonomy:

- `FormationDecision` — selected formation, rationale, confidence, experience key
- `FormationOutcome` — formation kind, suggestors used, fixed point reached,
  cycles used, extra loops used, quality score, forced fixed point

Add genome id, tournament id, lineage, provider assignments, and correlation id
to those records rather than forking the concept.

### Run Outcome Record

Extend ExperienceStore events to capture the full formation context:

- Formation template chosen
- Suggestor roster (with descriptor snapshots)
- Provider assignments per role
- Cycles to fixed point
- Stop reason
- Criteria passed/blocked
- Human gates triggered
- Final business quality score
- Downstream real-world outcome (when available)

**Storage strategy**: SurrealDB for structured run history, LanceDB for
similarity over past runs, graph recall for causal patterns like "these
provider mixes correlate with escalation" or "this formation shape works
for EU procurement but not US pricing."

The core terminal `OutcomeRecorded` event is useful but sparse. Higher layers
should not assume the kernel already records all tournament metrics such as
latency, tokens, or external business outcomes.

## HITL → Cedar Graduation

Humans disappear last, not first. Every HITL step becomes labeled data for
future Cedar policies, but only after stable inputs, stable reviewer
behavior, and acceptable false-positive/false-negative cost.

**Current substrate**: HITL gate decisions are now first-class
`ExperienceEvent::GateDecisionRecorded` records. That gives Organism a
canonical event payload for mining human decisions and drafting Cedar policy
candidates.

**Correlation boundary**: the generic `StoreObserver` currently wraps raw
engine events without tenant or correlation metadata. Formation-level
correlation belongs to Organism, not the engine. When Organism starts using
engine events for formation runs, prefer an Organism-owned observer that wraps
events with the active formation run's `tenant_id` and `correlation_id` before
appending them. Only extend `ExperienceEventObserver::on_event` if multiple
upper layers need the same metadata contract.

**Phase 1 — Human-only**: `ApprovalGateSuggestor` pauses convergence.
Every decision should be durably recorded with full proposal context and
rationale.

**Phase 2 — Pattern mining**: After N decisions on similar proposals, mine
the ExperienceStore for clusters (e.g., "always approve expense < $500
from domain:finance").

**Phase 3 — Draft Cedar policy**: Generate policy from the pattern:
```cedar
permit(
  principal in Role::"finance_analyst",
  action == Action::"approve_expense",
  resource
) when {
  resource.amount < 500
};
```

**Phase 4 — Shadow mode**: Run Cedar policy in parallel with HITL. Log
disagreements. If agreement > 99% over M decisions, propose graduation.

**Phase 5 — Graduate**: Replace `ApprovalGateSuggestor` with
`PolicyGateSuggestor`. Human moves to exception handling only.

**Phase 6 — Monitor**: If graduated policy causes convergence failures
(downstream agents can't resolve violations), automatically degrade back
to HITL and flag for review.

## OpenClaw: Bounded Exploration

`OpenClawFormationTemplate` exists with `max_extra_loops` but has no
execution semantics yet. OpenClaw should mean bounded exploration, not
freedom from governance.

### Execution semantics

1. Run baseline formation → reach Fixed Point A
2. Fork a **counterfactual branch** with a perturbation seed
3. Re-converge the branch → reach Fixed Point B
4. Compare A and B on outcome metrics
5. Repeat up to `max_extra_loops` with different perturbations
6. Return the Pareto-optimal set of fixed points

This should be implemented as branch exploration, not by polluting one truth
stream with contradictions. That keeps append-only truth and auditability
intact.

Analogous to simulated annealing — accept the current solution but
occasionally introduce energy to escape local optima.

### Mandatory guard rails

- **Required roles always present**: policy, evaluator, synthesis
- **Novelty budget**: how much structural deviation is allowed
- **Hard rails**: never bypass promotion, Cedar, budgets, or required HITL
- **Variant set**: provider swaps, extra adversarial/simulation members,
  alternate search/knowledge strategies

Every perturbation run is a separate convergence with its own
ExperienceStore trail. You're not bypassing governance — you're exploring
the solution space within governance.

### Advanced: formation-shape exploration

OpenClaw can explore not just different solutions to the same formation,
but different formation shapes for the same intent. "What if we added a
skeptic?" "What if we used a cheaper LLM and spent the budget on more
cycles?"

## Recall and tenancy boundaries

Keep runtime recall and offline learning separate.

- Runtime recall is for hinting and augmentation during a live run.
- Offline learning is for building priors, training datasets, and tournament
  selection.

The recall model already distinguishes these uses. Preserve that boundary so
Organism does not turn recall into hidden evidence or silent training data
exfiltration.

Default all prior mining, tournament analysis, and HITL policy graduation to
**tenant-scoped** data. Cross-tenant learning should be an explicit policy
decision, not an accident of a shared experience store.

## Concrete Priority Order

1. **Formation descriptor contract** — mandatory reads/writes metadata,
   domain tags, and backend requirements for formation-participating
   suggestors. Without this, both the compiler and the tournament are
   blind.
2. **FormationCompiler** — intent + three catalogs → executable
   FormationPlan. Set-cover assembly, per-role provider routing, policy
   preloading.
3. **Run Outcome Record** — extend ExperienceStore with full formation
   context. This is the training data for everything else.
4. **OpenClaw guard rails** — mandatory roles, novelty budget, hard rails.
   Bounded exploration, not anarchy.
5. **HITL→Cedar graduation pipeline** — shadow mode infrastructure first,
   then mining, then graduation.

## See Also

- [[Formation Pattern]] — current formation surface and embedding guide
- [[Suggestor Catalog]] — full inventory of available suggestors
- [[Experience and Recall]] — ExperienceStore contract and query surface
- [[HITL Gates]] — human-in-the-loop gate machinery
- [[Greedy Set Cover]] — the algorithm for complementary team assembly
- [[Formation Business Strategy]] — business strategy, go-to-market wedges, commercial framing
- [[Planning/Organism Formation Compiler Milestone Draft]] — broader Organism compiler milestone with vendor selection as first proof wedge
