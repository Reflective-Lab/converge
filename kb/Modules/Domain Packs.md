---
tags: [modules, domain]
source: mixed
---
# Domain Packs

`converge-domain` contains the built-in pack-organized Suggestors shipped with
this workspace.

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
use converge_domain::packs::delivery::RiskAssessorAgent;
use converge_domain::packs::trust::AuditWriterAgent;

engine.register_suggestor_in_pack("delivery", RiskAssessorAgent);
engine.register_suggestor_in_pack("trust", AuditWriterAgent);
```

Use pack labels when upper layers want to filter or assemble formations by
named pack. Use plain `register_suggestor(...)` when a suggestor does not
belong to a `converge-domain` pack.

## Same Loop, Different Crates

These crates are not domain packs, but they still participate through
`Suggestor` in the same engine run:

- `converge-policy`
- `converge-optimization`
- `prism` (extension repo — formerly `converge-analytics`)
- `mnemos` (extension repo — formerly `converge-knowledge`)

See also: [[Concepts/Domain Packs]], [[Architecture/Suggestor Contract]]
