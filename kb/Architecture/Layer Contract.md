---
tags: [architecture, contract, ecosystem]
---
# Layer Contract

Converge is the governed execution kernel.

Axiom defines what must be true. Organism decides how to form the team to satisfy it. Converge runs the governed loop, promotes facts, and records what actually happened. Product and hackathon apps own the user experience, artifact ingestion, integrations, and writeback.

## Firm Statement

Converge owns deterministic, auditable convergence.

It defines how suggestors propose evidence, how proposals become facts, how governance gates constrain promotion, how fixed-point execution stops, and how run experience is recorded. Converge is the authority boundary for operational truth: if a fact was not promoted by Converge, it is not a governed fact.

Converge does not define business truth contracts. That is Axiom.

Converge does not choose which formation should solve a business decision, run tournaments, learn formation priors, or own domain decision products. That is Organism.

Converge does not own UI, imported artifacts, customer-specific workflows, or writeback into systems of record. That belongs to product layers such as the hackathon app.

## Authority Split

```text
Axiom defines what must be true.
Organism decides how to form the team to satisfy it.
Converge governs execution and promotion of facts.
Product apps wire the experience, artifacts, and writeback.
```

## Converge Owns

- The `Suggestor` authoring contract and proposal-only agent effects.
- Context keys, proposals, facts, provenance, promotion, and criteria.
- The deterministic engine loop and honest stop reasons.
- HITL gates, policy snapshots, and governed promotion boundaries.
- Experience events emitted from governed execution.
- Public kernel/model/pack/provider contracts that upper layers can embed.
  The provider contract crate is `converge-provider`; adapter implementations
  carry qualified names such as `converge-provider-adapters`.

## Converge Must Not Own

- `.truths` authoring, validation, policy coverage analysis, or spec simulation.
- Formation compilation, set-cover team assembly, provider routing strategy, or tournaments.
- Business-domain outcome scoring or cross-run formation priors.
- Product-specific source ingestion, UX, approval screens, or system-of-record writeback.

## Integration Rule

Upper layers may use Converge to execute a governed decision, but they must not smuggle business authority into Converge internals. Business validity comes from Axiom, strategy comes from Organism, and operational facthood comes from Converge promotion.
