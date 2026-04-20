---
tags: [modules, domain]
source: mixed
---
# Domain Packs

`converge-domain` provides 47 production Suggestors across 7 business domains.
Registered via `register_suggestor_in_pack()` for pack-scoped activation.

## Domains

### Trust (8 agents)

| Agent | Purpose |
|-------|---------|
| AuditWriter | Records audit events |
| ComplianceScanner | Scans for compliance violations |
| ContractExecution | Executes contract terms |
| PiiRedactor | Redacts PII from proposals |
| ProvenanceTracker | Tracks data lineage |
| RbacEnforcer | Enforces role-based access |
| SessionValidator | Validates session integrity |
| ViolationRemediator | Proposes fixes for violations |

### Money (6 agents)

| Agent | Purpose |
|-------|---------|
| InvoiceCreator | Creates invoice proposals |
| InvoiceIssuer | Issues approved invoices |
| OverdueDetector | Flags overdue payments |
| PaymentAllocator | Allocates payments to invoices |
| PeriodCloser | Closes accounting periods |
| ReconciliationMatcher | Matches transactions |

### Delivery (9 agents)

| Agent | Purpose |
|-------|---------|
| AcceptanceRequestor | Requests stakeholder acceptance |
| BlockerDetector | Identifies delivery blockers |
| BlockerRouter | Routes blockers to owners |
| PostmortemScheduler | Schedules incident reviews |
| PromiseCreator | Creates delivery promises |
| RiskAssessor | Assesses delivery risks |
| ScopeExtractor | Extracts scope from requirements |
| StatusAggregator | Aggregates status across streams |
| WorkBreakdown | Decomposes work into tasks |

### Data Metrics (10 agents)

| Agent | Purpose |
|-------|---------|
| AlertEvaluator | Evaluates alert conditions |
| AnomalyDetector | Detects data anomalies |
| DashboardBuilder | Builds dashboard definitions |
| DataValidator | Validates data quality |
| FreshnessMonitor | Monitors data freshness |
| MetricCalculator | Computes business metrics |
| MetricRegistrar | Registers metric definitions |
| PipelineCoordinator | Coordinates data pipelines |
| ReportGenerator | Generates reports |
| SourceConnector | Connects to data sources |

### Meeting Scheduler (5 agents)

| Agent | Purpose |
|-------|---------|
| AvailabilityRetrieval | Gets participant availability |
| ConflictDetection | Detects scheduling conflicts |
| SlotOptimization | Finds optimal time slots |
| TimeZoneNormalization | Normalizes across time zones |
| WorkingHoursConstraint | Enforces working hours |

### Form Filler (7 agents)

| Agent | Purpose |
|-------|---------|
| Completeness | Checks form completeness |
| FieldMapping | Maps data to form fields |
| FillPlan | Plans form filling strategy |
| FormSchema | Parses form structure |
| Normalization | Normalizes field values |
| ProposalEmitter | Emits filled form proposals |
| RiskClassifier | Classifies form risk level |

### Resource Routing (5 agents)

| Agent | Purpose |
|-------|---------|
| ConstraintValidation | Validates routing constraints |
| Feasibility | Checks routing feasibility |
| ResourceRetrieval | Gets available resources |
| Solver | Solves routing optimization |
| TaskRetrieval | Gets tasks needing resources |

## Registration Pattern

```rust
use converge_domain::delivery::{RiskAssessor, WorkBreakdown};

engine.register_suggestor_in_pack("delivery", RiskAssessor);
engine.register_suggestor_in_pack("delivery", WorkBreakdown);
```

Pack registration enables selective activation via `TypesRootIntent`:
```rust
let intent = TypesRootIntent::new()
    .with_active_packs(vec!["delivery", "trust"]);
```

Only agents in active packs participate in that run.

See also: [[Architecture/Suggestor Contract]], [[Architecture/Formation Pattern]]
