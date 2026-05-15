---
tags: [moc, philosophy]
source: mixed
---
# Ecosystem — The Helicopter View

Converge is the governance layer in a five-layer stack built by Reflective Labs. Understanding where Converge sits — and what it does NOT do — is essential for keeping the architecture clean.

## The Five Layers

```
┌─────────────────────────────────────────────────────────────┐
│ Helm (control surface)                                      │
│ Desktop and web UX — what operators see                     │
├─────────────────────────────────────────────────────────────┤
│ Axiom (truth layer)                                         │
│ Truth definitions, projections, validation, domain state    │
├─────────────────────────────────────────────────────────────┤
│ Organism (intelligence)                                     │
│ Intent → Huddle → Debate → Suggestors                       │
│ The "how" — reasoning, research, gap-chasing                │
├─────────────────────────────────────────────────────────────┤
│ Converge (governance)                                       │
│ Engine, promotion gates, Cedar policy, budget, audit        │
│ The "whether" — authority, trust, stop rules                │
├─────────────────────────────────────────────────────────────┤
│ Providers (capability)                                      │
│ OpenRouter, Anthropic, OpenAI, Gemini, Brave, Tavily        │
└─────────────────────────────────────────────────────────────┘
```

## What Each Layer Owns

### Helm — The Control Surface

**Owns:**
- Operator-facing desktop or web experiences
- Commands, forms, timelines, and action surfaces
- Local bridges into the app layer

**Does NOT own:**
- Truth definitions or projections
- Reasoning strategy
- Promotion decisions

### Axiom — The Truth Layer

**Owns:**
- Application truth definitions
- Domain projections and view models
- Validation, scenario setup, and product-specific state
- The "what" a governed run is deciding

**Does NOT own:**
- Operator UX
- Governance policy enforcement
- Provider-specific clients in the UI

### Organism — The Thinking Layer

**Owns:**
- Intent interpretation — decomposing human goals into machine specs
- Multi-model collaborative planning (huddle loop)
- Adversarial governance — assumption breakers, constraint checkers, skeptics
- Simulation and gap-chasing
- Suggestor-level reasoning that feeds governed proposals

**Does NOT own:**
- Authority — recomputed at Converge's commit boundary
- Truth definitions and product projections
- The operator-facing control surface

### Converge — The Commit Boundary

**Owns:**
- The [[Philosophy/Nine Axioms|nine axioms]] — non-negotiable
- The [[Concepts/Proposals and Promotion|promotion gate]] — the only path from proposal to fact
- The [[Architecture/Engine Execution Model|convergence engine]] — 8-phase execution cycle
- [[Concepts/Context and Facts|Context, Facts, ProposedFacts]] — the type system
- [[Concepts/Invariants|Invariants]] — executable guarantees
- [[Concepts/Domain Packs|Domain packs]] — trust, money, delivery, knowledge, data_metrics
- Cedar-backed policy and authority evaluation
- Budgets, stop rules, traceability, and audit

**Does NOT own:**
- How plans are made (that is Organism)
- What the user sees (that is Helm)
- Product-specific truth definitions and domain state (that is Axiom)

### Providers — The Capability Layer

**Owns:**
- External LLM, search, storage, optimization, and tool adapters
- Concrete implementations behind `ChatBackend`, `WebSearchBackend`, `DdLlm`, `DdSearch`, and MCP-style surfaces

**Does NOT own:**
- Governance decisions
- Product workflows
- UI-facing business logic

**Critical invariant:** Authority is never inherited from reasoning. Organism may produce `ProposedFact` and `AgentEffect`; Converge alone decides what becomes fact.

Products like Wolfgang and Monterro usually own both Helm and Axiom. The naming split is intentional: what operators touch is not the same as what defines truth.

## The Flow

```
Axiom defines truth, projections, and run configuration
    ↓
Axiom invokes `Engine.run()`
    ↓
Organism decomposes → plans → debates → researches → emits proposals
    ↓
Converge evaluates policy, budget, convergence, and promotion
    ↓
Providers supply capability calls behind adapter boundaries
    ↓
Axiom projects converged facts into product state
    ↓
Helm shows the result to the operator
```

Providers never decide. Organism never self-authorizes. Helm never talks to providers directly.

## The Dependency Rule

```
Helm depends on Axiom
Axiom depends on Organism and/or Converge
Organism depends on Converge
Converge depends on provider capability adapters below it
Providers depend on the outside world, not on application layers above them
```

This is the hexagonal architecture applied at the organizational level. Converge defines [[Architecture/Ports|ports]]. Organism and product layers are adapters. Converge never imports from anything above it.

When a product needs a capability that doesn't exist:
1. If it's a governance/convergence capability → build it in **Converge**
2. If it's a reasoning/planning capability → build it in **Organism**
3. If it's truth modeling or projection → build it in **Axiom**
4. If it's control-surface specific → build it in **Helm**

Never work around Converge. Patch it.

See also: [[Philosophy/Why Converge]], [[Philosophy/Nine Axioms]], [[Architecture/Hexagonal Architecture]]
