---
tags: [concepts]
source: mixed
---
# Domain Packs

Domain packs are **one** way of organizing built-in suggestors. They are not
the only way to participate in convergence.

## Built-In Packs (atelier showcase, formerly `converge-domain`)

These packs moved to the **atelier** showcase repo (`~/dev/atelier`) on
2026-05-05. They are exemplars, not foundation contracts.

| Pack | Purpose | Representative Suggestors |
|---|---|---|
| `trust` | Audit, access control, provenance, compliance | `SessionValidatorAgent`, `RbacEnforcerAgent`, `AuditWriterAgent`, `PiiRedactorAgent` |
| `money` | Invoicing, allocation, reconciliation, close | `InvoiceCreatorAgent`, `PaymentAllocatorAgent`, `ReconciliationMatcherAgent`, `PeriodCloserAgent` |
| `delivery` | Promise fulfillment, blockers, status, risk | `PromiseCreatorAgent`, `WorkBreakdownAgent`, `BlockerDetectorAgent`, `RiskAssessorAgent` |
| `data_metrics` | Metrics, data quality, dashboards, alerts | `MetricRegistrarAgent`, `DataValidatorAgent`, `DashboardBuilderAgent`, `AlertEvaluatorAgent` |

Register built-in packs with `register_suggestor_in_pack(...)` when you want
pack labels for filtering or formation assembly:

```rust
engine.register_suggestor_in_pack("trust", AuditWriterAgent);
engine.register_suggestor_in_pack("delivery", RiskAssessorAgent);
```

## Other Suggestor Families in the Same Loop

Not every Suggestor belongs in an atelier domain pack.

| Crate | Role in the loop |
|---|---|
| `arbiter` (extension, formerly `converge-policy`) | Policy and flow gates as Suggestors. Lives in `~/dev/reflective/mosaic-extensions/arbiter`. |
| `converge-optimization` | Solver packs through `SolverSuggestor<P>` |
| `prism` (extension, formerly `converge-analytics`) | Feature extraction, training, inference, monitoring suggestors. Lives in `~/dev/reflective/mosaic-extensions/prism`. |
| `mnemos` (extension, formerly `converge-knowledge`) | Retrieval and persistence suggestors for the knowledge base. Lives in `~/dev/reflective/mosaic-extensions/mnemos`. |

These are still plain `Suggestor`s. They register through the same engine API
and run in the same convergence loop.

## Mixing Built-In Packs with Other Suggestors

```rust
engine.register_suggestor_in_pack("delivery", RiskAssessorAgent);
engine.register_suggestor(policy_gate);
engine.register_suggestor(solver);
engine.register_suggestor(knowledge_retrieval);
engine.register_suggestor(custom_llm);
```

That is the intended architecture. One loop. One contract.

## Ownership Boundary

- Converge owns the loop and the governance mechanics.
- Formation assembly belongs to the consumer layer.
- Organism chooses which packs and suggestors belong in a formation.
- Helms and applications decide when to run that formation.

See also: [[Architecture/Suggestor Contract]], [[Building/Crate Catalog]]
