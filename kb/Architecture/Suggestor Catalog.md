---
tags: [architecture, catalog, contract]
source: mixed
---
# Suggestor Catalog

Complete inventory of all Suggestors in the Converge workspace. One contract,
one loop, ~118 agents.

## Optimization (21 Suggestors via `SolverSuggestor<Pack>`)

All accessed through `converge_optimization::suggestor::SolverSuggestor::new(Pack, input_key, output_key)`.

### Choose Items

| Pack | Purpose | Solver |
|------|---------|--------|
| BudgetAllocationPack | Allocate budget across categories maximizing ROI | Efficiency-based allocation |
| BacklogPrioritizationPack | Prioritize backlog items by value/effort | Weighted scoring |
| VendorShortlistPack | Select best vendors from candidates | Multi-criteria ranking |
| PricingGuardrailsPack | Constrain pricing within guardrails | Bound enforcement |
| AnomalyTriagePack | Triage and prioritize anomaly alerts | Severity-based ranking |

### Assign Resources

| Pack | Purpose | Solver |
|------|---------|--------|
| AssignmentPack | Assign N agents to N tasks minimizing cost | Hungarian algorithm |
| LeadRoutingPack | Route leads to sales reps by fit | Weighted matching |
| MeetingSchedulerPack | Schedule meetings across availability/time zones | Constraint satisfaction |
| CapacityPlanningPack | Plan resource capacity against demand forecast | LP relaxation |
| JobShopSchedulingPack | Schedule jobs on machines minimizing makespan | Priority dispatch (SPT) |
| StaffRosteringPack | Assign staff to shifts respecting skills/regulations | Greedy skill-matching + load balancing |

### Move Things

| Pack | Purpose | Solver |
|------|---------|--------|
| VehicleRoutingPack | Route vehicles to customers with capacity constraints | Nearest-neighbor heuristic |
| ShippingChoicePack | Select optimal shipping method per shipment | Cost optimization |
| NetworkFlowPack | Min-cost flow through a network | Successive shortest path (Bellman-Ford) |
| InventoryReplenishmentPack | Replenish inventory based on stock levels | Reorder-point |
| InventoryRebalancingPack | Rebalance inventory across warehouses | Transfer optimization |
| TravelingSalesmanPack | Shortest tour visiting all cities | Nearest-neighbor + 2-opt |

### Decide Locations

| Pack | Purpose | Solver |
|------|---------|--------|
| FacilityLocationPack | Choose facility locations minimizing total cost | Greedy opening |

### Split Groups

| Pack | Purpose | Solver |
|------|---------|--------|
| BinPackingPack | Pack items into bins minimizing bins used | First-fit decreasing |
| GraphPartitioningPack | Partition graph into balanced subsets | Degree-based + Kernighan-Lin |

### General

| Pack | Purpose | Solver |
|------|---------|--------|
| ConstraintProgrammingPack | Any constraint problem — universal solver | Backtracking + propagation |

---

## Analytics — ML Pipeline Agents (11 Suggestors — extracted to prism)

Direct `Suggestor` implementations now in the **prism** extension crate
(`~/dev/reflective/stack/mosaic-extensions/prism`). Extracted from `converge-analytics` on 2026-05-05.

| Agent | Purpose | Watches | Writes |
|-------|---------|---------|--------|
| FeatureAgent | Polars-based feature extraction | Seeds | Proposals |
| DataValidationAgent | Validates input data quality | Signals | Signals |
| FeatureEngineeringAgent | Computes ML features | Signals | Constraints |
| HyperparameterSearchAgent | Optimizes model parameters | Constraints, Signals | Constraints, Evaluations |
| DatasetAgent | Prepares training datasets | Seeds | Signals |
| ModelTrainingAgent | Trains ML models | Signals | Strategies |
| ModelEvaluationAgent | Evaluates model quality (MAE) | Strategies, Evaluations | Evaluations |
| ModelRegistryAgent | Registers passing models | Evaluations, Strategies | Signals |
| MonitoringAgent | Tracks model drift | Signals, Strategies | Strategies |
| DeploymentAgent | Deploys validated models | Evaluations, Strategies | Signals, Strategies |
| SampleInferenceAgent | Demo inference for validation | Signals, Strategies | Strategies |

## Analytics — Data Packs (9 Suggestors via `SolverSuggestor<Pack>` — extracted to prism)

Live in the **prism** extension (`~/dev/reflective/stack/mosaic-extensions/prism`, formerly
`converge-analytics`). Accessed through
`converge_optimization::suggestor::SolverSuggestor::new(AnalyticsPack, input_key, output_key)`.

| Pack | Purpose | Default: Input → Output |
|------|---------|-------------------------|
| AnomalyDetectionPack | Statistical anomaly detection | Seeds → Strategies |
| ClassificationPack | Categorical classification | Seeds → Strategies |
| DescriptiveStatsPack | Summary statistics | Seeds → Strategies |
| ForecastingPack | Time-series forecasting | Seeds → Strategies |
| RankingPack | Item ranking/scoring | Seeds → Strategies |
| RegressionPack | Numeric regression | Seeds → Strategies |
| SegmentationPack | Customer/data segmentation | Seeds → Strategies |
| SimilarityPack | Similarity matching | Seeds → Strategies |
| TrendDetectionPack | Trend identification | Seeds → Strategies |

---

## Policy Gates (8 Suggestors — extracted to arbiter)

Direct `Suggestor` implementations now in the **arbiter** extension crate
(`~/dev/reflective/stack/mosaic-extensions/arbiter`). Extracted from `converge-policy` on 2026-05-05.

| Gate | Purpose | Watches | Writes |
|------|---------|---------|--------|
| PolicyGateSuggestor | Cedar policy evaluation | configurable | Constraints |
| DelegationVerifySuggestor | Ed25519 delegation chain verification | configurable | Constraints |
| FlowGateSuggestor | Flow-level authorization | configurable | Constraints |
| RateLimitGateSuggestor | Throttle proposals per key per run | configurable | Constraints |
| BudgetGateSuggestor | Cost/token budget enforcement mid-convergence | configurable | Constraints |
| ApprovalGateSuggestor | HITL approval for high-stakes proposals | configurable | Constraints |
| DataClassificationGateSuggestor | Block PII (email, SSN, credit card, phone) | configurable | Constraints |
| ComplianceGateSuggestor | GDPR/SOC2/HIPAA rule evaluation | configurable | Constraints |

---

## Knowledge (2 Suggestors — extracted to mnemos)

Direct `Suggestor` implementations now in the **mnemos** extension crate
(`~/dev/reflective/stack/mosaic-extensions/mnemos`). Extracted from `converge-knowledge` on 2026-05-05.

| Agent | Purpose | Watches | Writes |
|-------|---------|---------|--------|
| KnowledgeRetrievalSuggestor | Search KB, propose relevant knowledge | Seeds | Hypotheses |
| KnowledgeStoreSuggestor | Store convergence results for future use | Evaluations | Seeds (confirmation) |

---

## Domain Packs (47 Suggestors — moved to atelier)

Registered via `register_suggestor_in_pack()` in the **atelier** showcase
repo (`~/dev/atelier`). Moved from `converge-domain` on 2026-05-05.

### Trust (8)

| Agent | Purpose |
|-------|---------|
| AuditWriterAgent | Records audit events |
| ComplianceScannerAgent | Scans for compliance violations |
| ContractExecutionAgent | Executes contract terms |
| PiiRedactorAgent | Redacts PII from proposals |
| ProvenanceTrackerAgent | Tracks data lineage |
| RbacEnforcerAgent | Enforces role-based access |
| SessionValidatorAgent | Validates session integrity |
| ViolationRemediatorAgent | Proposes fixes for violations |

### Money (6)

| Agent | Purpose |
|-------|---------|
| InvoiceCreatorAgent | Creates invoice proposals |
| InvoiceIssuerAgent | Issues approved invoices |
| OverdueDetectorAgent | Flags overdue payments |
| PaymentAllocatorAgent | Allocates payments to invoices |
| PeriodCloserAgent | Closes accounting periods |
| ReconciliationMatcherAgent | Matches transactions |

### Delivery (9)

| Agent | Purpose |
|-------|---------|
| AcceptanceRequestorAgent | Requests stakeholder acceptance |
| BlockerDetectorAgent | Identifies delivery blockers |
| BlockerRouterAgent | Routes blockers to owners |
| PostmortemSchedulerAgent | Schedules incident reviews |
| PromiseCreatorAgent | Creates delivery promises |
| RiskAssessorAgent | Assesses delivery risks |
| ScopeExtractorAgent | Extracts scope from requirements |
| StatusAggregatorAgent | Aggregates status across streams |
| WorkBreakdownAgent | Decomposes work into tasks |

### Data Metrics (10)

| Agent | Purpose |
|-------|---------|
| AlertEvaluatorAgent | Evaluates alert conditions |
| AnomalyDetectorAgent | Detects data anomalies |
| DashboardBuilderAgent | Builds dashboard definitions |
| DataValidatorAgent | Validates data quality |
| FreshnessMonitorAgent | Monitors data freshness |
| MetricCalculatorAgent | Computes business metrics |
| MetricRegistrarAgent | Registers metric definitions |
| PipelineCoordinatorAgent | Coordinates data pipelines |
| ReportGeneratorAgent | Generates reports |
| SourceConnectorAgent | Connects to data sources |

### Meeting Scheduler (5)

| Agent | Purpose |
|-------|---------|
| AvailabilityRetrievalAgent | Gets participant availability |
| ConflictDetectionAgent | Detects scheduling conflicts |
| SlotOptimizationAgent | Finds optimal time slots |
| TimeZoneNormalizationAgent | Normalizes across time zones |
| WorkingHoursConstraintAgent | Enforces working hours |

### Form Filler (7)

| Agent | Purpose |
|-------|---------|
| CompletenessAgent | Checks form completeness |
| FieldMappingAgent | Maps data to form fields |
| FillPlanAgent | Plans form filling strategy |
| FormSchemaAgent | Parses form structure |
| NormalizationAgent | Normalizes field values |
| ProposalEmitterAgent | Emits filled form proposals |
| RiskClassifierAgent | Classifies form risk level |

### Resource Routing (5)

| Agent | Purpose |
|-------|---------|
| ConstraintValidationAgent | Validates routing constraints |
| FeasibilityAgent | Checks routing feasibility |
| ResourceRetrievalAgent | Gets available resources |
| SolverAgent | Solves routing optimization |
| TaskRetrievalAgent | Gets tasks needing resources |

---

## Core Engine (internal/test, ~20)

| Agent | Purpose |
|-------|---------|
| SeedSuggestor | Seeds initial facts |
| ReactOnceSuggestor | Single-cycle reactor |
| StrategyAgent | Derives strategies from hypotheses |
| DeriverAgent | Derives from existing facts |
| ValidationAgent | Validates context |
| AlwaysAgent, FloodAgent, InfiniteAgent... | Test harness agents |

---

## Formation Composition Examples

Formations mix agents from any module. Common patterns:

| Formation | Agents composed | Use case |
|-----------|----------------|----------|
| Budget Planning | BudgetAllocationPack + PolicyGate + LLM Reasoning | Annual budget allocation with compliance |
| Incident Response | AnomalyDetectionPack + AnomalyTriagePack + CapacityPlanningPack | SRE alert → triage → scale |
| Sales Operations | SegmentationPack + LeadRoutingPack + PolicyGate | Segment → route → constrain |
| Supply Chain | ForecastingPack + InventoryReplenishmentPack + BudgetAllocationPack | Forecast → replenish → budget |
| Procurement | ClassificationPack + RankingPack + VendorShortlistPack | Classify → rank → select |
| Due Diligence | KnowledgeRetrieval + LLM Research + DataClassificationGate | Search → reason → redact PII |
| Staff Scheduling | StaffRosteringPack + PolicyGate + ApprovalGate | Roster → enforce regs → HITL approval |
| Route Optimization | VehicleRoutingPack + BudgetGate + ComplianceGate | Route → budget check → compliance |

All agents register through `engine.register_suggestor()`. Same contract,
same governance, same convergence loop.

See also: [[Suggestor Contract]], [[Formation Pattern]]
