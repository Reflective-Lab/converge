---
tags: [planning, organism, formations, vendor-selection, milestone]
source: mixed
---
# Organism Formation Compiler Milestone Draft

Organism-specific planning companion to [[Architecture/Formation Building Review]]
and [[Planning/Formation Business Strategy]]. This page translates the
Converge-side architecture review, business-strategy review, and latest
Converge implementation handoff into a broader Organism milestone where
**vendor selection is the first proof wedge**.

This is a **draft planning note**, not yet the committed milestone in
`[[Planning/MILESTONES]]`.

## Milestone Claim

**Organism can compile a cross-functional business intent into a governed,
explainable formation, run it through Converge, capture the outcome, and use
that evidence to improve future formation choice.**

Vendor selection is the first proof wedge because it is reconciliation-heavy,
policy-rich, and visible enough to validate the architecture during the
hackathon. The milestone should remain broader than procurement: it is about
building the reusable Organism formation compiler, with vendor selection as the
first decision product that proves it.

## Why This Exists

The architecture review says what the next layer must do technically:

- discover capabilities through explicit descriptors
- compile formations from three catalogs
- route providers per role
- keep planning, execution, recall, and governance cleanly separated
- record outcomes strongly enough to support learning and policy graduation

The business review says what must be proven commercially:

- kill the reconciliation tax
- replace the reconciliation meeting with a governed decision process
- start with one painful, policy-heavy wedge
- measure business quality and trust, not just convergence speed

This page answers the Organism question:

**If Converge is the governed execution kernel, what should Organism build
first so the result is both technically correct and commercially undeniable?**

## Converge Substrate Handoff

Converge now exposes a better substrate for Organism's formation layer while
preserving the boundary: Converge executes and records governed convergence;
Organism decides which formation to compile, run, compare, and learn from.

### Organism Can Now Use

- `GateDecisionRecorded` in the experience vocabulary, so HITL decisions can
  become learning input for future policy graduation.
- `Engine::resume()` gate-id mismatch rejection before promotion/event
  emission, so Organism can trust gate resumes not to cross wires.
- `FormationDecision.correlation_id` and `FormationOutcome.correlation_id`, so
  Organism has a join key across planning run, execution run, tournament
  candidate, and outcome record.
- `ProviderRequest.backend_requirements`, so Organism can express richer
  role-level routing needs instead of only flat capability lists.
- `converge-kernel` re-exports the canonical HITL and formation surfaces
  Organism should import from; Organism should not import `converge-core`
  directly for these paths.

### Sharp Edges To Preserve

- `StoreObserver` still creates envelopes without tenant/correlation. When
  Organism uses engine event observation for formation runs, prefer an
  Organism-owned observer that wraps events with the formation run's
  `tenant_id` and `correlation_id` before appending. The Converge engine does
  not need to know formation-level correlation; that is Organism's orchestration
  concern unless multiple upper layers later need a richer observer contract.
- `backend_requirements` is public, but current routing only enforces what the
  `Backend` trait exposes: kind, capabilities, replay, and offline. Cost,
  latency, sovereignty, and compliance can be carried now, but not fully
  enforced yet.
- Capability discovery is still not automatic. Organism needs its own
  descriptor inventory until Converge has a mandatory formation descriptor or
  manifest path.
- Any Organism work that depends on these surfaces must depend on a Converge
  worktree/commit/release that contains them.

## Core Thesis

The first Organism milestone should not be generic formation learning, and it
should not be only a vendor-selection app.

It should be:

**a reusable FormationCompiler validated through vendor selection.**

That shape keeps the work strategically aligned:

- broad enough to become the Organism formation layer
- narrow enough to validate through one real business wedge
- technical enough to prove compiler/catalog/provider/outcome contracts
- commercial enough to show why the system is better than today's process

## Why Vendor Selection Is The First Wedge

Vendor selection is not just a procurement workflow. It is a recurring,
cross-functional decision that forces a business to reconcile:

- commercial fit
- technical capability
- compliance and security posture
- legal and contractual constraints
- financial terms and total cost
- implementation and operating risk

It also aligns with the current hackathon: buyer-side learning from live vendor
evaluation is better source material than abstract architecture debate.

## What The Hackathon Should Teach Organism

The hackathon is field discovery for the first formation archetype. Capture at
least these inputs:

- how buyers enter the process
- who participates in the decision and who only approves
- what evidence sources matter in practice
- what compliance and security checks are mandatory
- which evaluation criteria are explicit vs tacit
- where the current process is simplified just to stay manageable
- what systems hold source evidence and which system should receive writeback
- what counts as a good downstream outcome

The critical insight to preserve:

**The current human process is not the ideal process. It is the process humans
can afford to run under coordination limits.**

Organism should not merely automate today's shortcuts. It should use Converge's
capabilities to restore diligence that humans currently compress away because
the full process is too expensive to run manually.

## What "Better Than Today's Process" Means

The goal is not only faster vendor selection. It is a stronger decision process
with fewer forced simplifications.

| Today's simplification | Why humans do it | What Organism should restore |
|---|---|---|
| Early shortlist collapse | Too many vendors and too much evidence to compare deeply | Wider evidence collection with explicit reasons for narrowing |
| Flattened scoring rubric | Cross-functional criteria are hard to reconcile in meetings | Structured criteria with visible weights, conflicts, and rationale |
| Shallow compliance pass | Security, privacy, and legal review are expensive to coordinate | Explicit governed checks with traceable pass, fail, and escalation paths |
| One-size-fits-all analysis | Teams use the same crude process for all categories | Formation compilation that adapts to domain, risk class, and materiality |
| Generic tool usage | One analyst or one model does all the work | Per-role provider routing and complementary specialist suggestors |
| No counterfactual comparison | Comparing alternatives thoroughly takes too long | Bounded scenario exploration for near-miss vendors or alternate criteria mixes |
| Decision memory in people's heads | Outcomes rarely get attached back to the decision | Experience-backed recall with actual downstream results |

## Organism's Responsibility

Converge remains the governed execution layer. Organism owns the
decision-shaping layer above it.

| Concern | Converge owns | Organism owns |
|---|---|---|
| Execution and promotion | fixed-point convergence, proposals, authority, budgets, governance | choosing the formation to run |
| Formation semantics | stable request and outcome contracts | archetype selection and formation compilation |
| Suggestor/provider primitives | public capability and execution surfaces | descriptor catalogs and selection strategy |
| HITL and Cedar mechanics | gate execution and policy enforcement | graduation strategy and policy candidate mining |
| Experience plumbing | durable events, recall boundaries, tenant scoping | priors, tournament selection, and decision memory use |
| Business wedge expression | generic governed loop | named decision products such as vendor selection |

Organism's deliverable is therefore not "more suggestors." It is:

**a compiler that turns a business intent plus evidence into an executable,
governed, explainable formation plan.**

## Application Builder Handoff

The next application work across `../axiom`, `../organism`, and Converge should
keep the layer split explicit:

- Application surfaces and operator workflows should enter through Helm or the
  app layer, then call Organism for planning.
- Organism should compile formation plans, choose providers per role, manage
  correlation, and own outcome learning.
- Axiom should provide typed truth definitions, projections, validations, and
  scenario/domain state that Organism can reference and Converge can execute
  against.
- Converge should stay the governed execution kernel: proposal emission,
  promotion, policy gates, budgets, HITL mechanics, event capture, and public
  embedding contracts.

The implementation rule is simple:

**planning and learning live above Converge; authority and audit stay in
Converge.**

## Candidate Milestone

**Candidate name:** Organism Formation Compiler

**Candidate theme:** Build the reusable Organism compiler that turns a
business intent into a governed Converge formation, using vendor selection as
the first proof wedge.

### Scope

- three catalogs: formation templates, suggestor descriptors, provider
  descriptors
- one compiler path from business intent to executable formation plan
- one proof wedge: vendor selection
- one HITL-first approval path with experience capture
- one correlation strategy across planning, execution, and outcome
- one durable outcome record for later learning

### Non-goals

- generic tournamenting across many archetypes
- broad autonomous approvals
- cross-tenant learning
- replacing systems of record
- proving every OpenClaw scenario pattern in the first cut
- shipping the whole "platform for everything"
- moving `FormationGenome` into `converge-model`

## Draft Validation Points

The milestone should only count as complete if it proves both the reusable
compiler and the vendor-selection wedge.

### 1. Formation Catalogs Are Real

- Formation templates, suggestor descriptors, and provider descriptors are
  separate catalogs
- Suggestor descriptors include reads, output keys, domain tags, capabilities,
  provider needs, cost/latency hints, and confidence range
- Provider descriptors use public `converge-provider-api` vocabulary rather
  than internal Converge registries

### 2. Organism Compiles Formations

- Organism compiles a vendor-selection intent into an executable formation plan
- Assembly is complementary team selection, not the current "every member must
  match every capability" filter
- Planning convergence is separate from the business convergence it launches
- The compiler emits an explainable formation decision with `correlation_id`

### 3. Provider Routing Is Per Role

- Research, extraction, synthesis, and policy roles can receive different
  provider requirements
- Role-level routing uses `BackendRequirements` where available
- Provider choice is explainable in the final decision trail
- The milestone records which backend constraints are carried vs actually
  enforceable today

### 4. Vendor Selection Archetype Is Typed

- Vendor selection exists as a first-class formation archetype/template
- Intake fields, entities, criteria, risk classes, evidence sources, and
  approval actors are typed rather than left as loose prompts
- Hackathon findings are captured as structured inputs, not only narrative
  notes

### 5. Compliance And Evaluation Are First-Class

- Security, privacy, compliance, finance, and contractual checks appear as
  explicit governed steps
- Evaluation criteria are traceable, weighted, and reviewable
- Escalations and missing evidence are visible in the decision record

### 6. The Decision Produces A Real Record

- Final recommendation, rationale, evidence, policy state, and approvals are
  captured together
- The decision writes back to the relevant downstream system or artifact
- Recommendation-only demos do not count as done

### 7. Outcomes Become Learning Data

- Experience records include template choice, roster, provider assignments,
  stop reason, gate triggers, and downstream business outcome fields
- `correlation_id` joins planning, execution, and outcome records
- Tenant scoping stays explicit
- Runtime recall and offline learning remain separate

### 8. HITL-To-Cedar Has A Bridge

- `GateDecisionRecorded` events are queryable for the vendor-selection wedge
- Organism captures enough proposal context and reviewer rationale for policy
  mining later
- Shadow-policy comparison is possible where patterns are stable enough to test
- Graduation logic stays in Organism, not Converge

### 9. The Process Is Better, Not Merely Faster

- The system demonstrably considers evidence or criteria that the current human
  process usually compresses away
- The output shows fewer forced shortcuts, not just a faster version of the
  same simplification
- Success metrics include business quality and risk reduction, not just
  cycle-time

## What The Hackathon Should Produce

Treat the hackathon as milestone input generation.

Desired outputs:

- a map of the current buyer-side vendor-selection flow
- the recurring evidence sources and artifacts
- the actual compliance and risk checks used in practice
- the scoring criteria and who owns them
- the points where humans simplify or skip diligence to stay on schedule
- the systems of record that need read and write integration
- the downstream outcomes that matter enough to learn from later

Those findings should feed directly into:

- the vendor-selection template
- the suggestor descriptor inventory
- the per-role provider requirements
- the policy and HITL design
- the outcome record schema

## Early Success Metrics

This wedge should be evaluated on three axes.

### Business

- reconciliation hours removed
- time from intake to approved decision
- number of stakeholder meetings collapsed or shortened
- downstream vendor outcome quality

### Governance

- approval latency
- policy violations prevented
- escalation rate
- audit completeness

### Learning

- how often prior cases are reusable
- how often criteria or policies require revision
- HITL-to-shadow-policy agreement
- ability to replay and explain the decision path

## Planning Recommendation

Do not update the top-level milestone to a vague "formation intelligence"
label.

Instead, plan the next Organism milestone around a claim that can be validated:

**Organism can compile governed formations for cross-functional business
decisions, with vendor selection as the first proof wedge.**

If that claim becomes true, the broader tournament, OpenClaw, and
HITL-to-Cedar roadmap becomes much easier to justify.

## See Also

- [[Architecture/Formation Building Review]] — architecture constraints and the compiler shape
- [[Planning/Formation Business Strategy]] — business framing, wedges, and trust-building autonomy
- [[Architecture/Formation Pattern]] — current Converge formation surface
- [[Concepts/Experience and Recall]] — learning and recall boundaries
- [[Concepts/HITL Gates]] — human approval and escalation mechanics
- [[Algorithms/Greedy Set Cover]] — complementary team assembly logic
- [[Architecture/Golden Path Matrix]] — layer ownership and public surfaces
