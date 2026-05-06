---
tags: [modules, domain]
source: mixed
---
# Domain Packs

> **Moved to atelier on 2026-05-05.** The built-in domain packs (and
> previously the `crates/domain` crate) now live in the **atelier** showcase
> repo (`~/dev/atelier`, formerly `converge-domain`). Foundation no longer
> ships built-in domain packs. See [[Architecture/Extension Topology]].

The atelier showcase contains pack-organized Suggestors as worked exemplars.

## Built-In Packs

### Trust

Audit, provenance, RBAC, compliance, and redaction.

Representative suggestors:
- `SessionValidatorAgent`
- `RbacEnforcerAgent`
- `AuditWriterAgent`
- `ProvenanceTrackerAgent`
- `ComplianceScannerAgent`
- `ViolationRemediatorAgent`
- `ContractExecutionAgent`
- `PiiRedactorAgent`

### Money

Invoice creation, issuance, allocation, reconciliation, and close.

Representative suggestors:
- `InvoiceCreatorAgent`
- `InvoiceIssuerAgent`
- `PaymentAllocatorAgent`
- `ReconciliationMatcherAgent`
- `OverdueDetectorAgent`
- `PeriodCloserAgent`

### Delivery

Work decomposition, blockers, risk, status, and acceptance.

Representative suggestors:
- `PromiseCreatorAgent`
- `ScopeExtractorAgent`
- `WorkBreakdownAgent`
- `BlockerDetectorAgent`
- `BlockerRouterAgent`
- `RiskAssessorAgent`
- `StatusAggregatorAgent`
- `AcceptanceRequestorAgent`
- `PostmortemSchedulerAgent`

### Data Metrics

Metrics, freshness, anomalies, dashboards, reports, and alerts.

Representative suggestors:
- `MetricRegistrarAgent`
- `SourceConnectorAgent`
- `PipelineCoordinatorAgent`
- `DataValidatorAgent`
- `AnomalyDetectorAgent`
- `DashboardBuilderAgent`
- `ReportGeneratorAgent`
- `AlertEvaluatorAgent`
- `FreshnessMonitorAgent`
- `MetricCalculatorAgent`

## Registration Pattern

```rust
use atelier_domain::packs::delivery::RiskAssessorAgent;
use atelier_domain::packs::trust::AuditWriterAgent;

engine.register_suggestor_in_pack("delivery", RiskAssessorAgent);
engine.register_suggestor_in_pack("trust", AuditWriterAgent);
```

Use pack labels when upper layers want to filter or assemble formations by
named pack. Use plain `register_suggestor(...)` when a suggestor does not
belong to an atelier pack.

## Same Loop, Different Crates

These crates are not domain packs, but they still participate through
`Suggestor` in the same engine run:

- `converge-optimization`
- `arbiter` (extension repo — formerly `converge-policy`)
- `prism` (extension repo — formerly `converge-analytics`)
- `mnemos` (extension repo — formerly `converge-knowledge`)

See also: [[Concepts/Domain Packs]], [[Architecture/Suggestor Contract]]
