---
tags: [architecture, analytics, packs, suggestors]
source: mixed
---
# Analytics Packs

> **Moved to prism extension on 2026-05-05.** Analytics packs now live in the
> **prism** extension repo (`~/dev/extensions/prism`, formerly
> `converge-analytics`). The file paths below describe the prism workspace
> layout. See [[Architecture/Extension Topology]].

Prepackaged Suggestors for common data analytics and machine learning problems.
Each pack implements the `Pack` trait from `converge-optimization` and participates
in formations via `SolverSuggestor<P>`.

## Available Packs

| Pack | Struct | Solver | What it does |
|------|--------|--------|-------------|
| anomaly-detection | `AnomalyDetectionPack` | Z-score thresholding | Flag outliers in numeric data |
| segmentation | `SegmentationPack` | K-means clustering | Group records into k clusters |
| ranking | `RankingPack` | Weighted multi-criteria scoring | Order items by composite score |
| forecasting | `ForecastingPack` | Exponential smoothing | Predict future values with confidence intervals |
| classification | `ClassificationPack` | Logistic (sigmoid) | Assign category labels with probability |
| regression | `RegressionPack` | Linear regression | Predict numeric values |
| similarity | `SimilarityPack` | Pairwise distance (cosine/euclidean/manhattan) | Find nearest-neighbor pairs |
| trend-detection | `TrendDetectionPack` | Moving-average slope analysis | Detect trends and changepoints |
| descriptive-stats | `DescriptiveStatsPack` | Summary statistics | Mean, median, std, percentiles, skewness, kurtosis |

## Usage

### Standalone (single pack in a formation)

```rust
use converge_kernel::{Budget, ContextKey, ContextState, Engine};
use converge_optimization::suggestor::SolverSuggestor;
use converge_analytics::AnomalyDetectionPack;

let mut engine = Engine::with_budget(Budget { max_cycles: 5, max_facts: 100 });
engine.register_suggestor(SolverSuggestor::new(
    AnomalyDetectionPack,
    ContextKey::Seeds,
    ContextKey::Strategies,
));

let mut ctx = ContextState::new();
ctx.add_input(ContextKey::Seeds, "input-1", serde_json::json!({
    "values": [10.0, 11.0, 50.0, 10.5, 100.0],
    "threshold": 2.0
}).to_string());

let result = engine.run(ctx).await?;
```

### Factory functions

```rust
use converge_analytics::suggestor;

engine.register_suggestor(suggestor::anomaly_detection());
engine.register_suggestor(suggestor::forecasting());
engine.register_suggestor(suggestor::classification());
// etc.
```

## Formation Patterns

Analytics packs are most powerful when composed with other suggestors in
multi-agent formations. Each pack reads from one ContextKey and writes to
another, enabling dependency-driven sequencing across cycles.

### Pattern: Detect → Triage → Act

```
anomaly_detection (Seeds → Strategies)
    ↓ next cycle
anomaly_triage (Strategies → Evaluations)     [optimization pack]
    ↓ next cycle
capacity_planning (Evaluations → Hypotheses)  [optimization pack]
```

Use case: **SRE alerting** — detect anomalous metrics, triage by severity,
propose capacity changes.

### Pattern: Segment → Route → Constrain

```
segmentation (Seeds → Strategies)
    ↓ next cycle
lead_routing (Strategies → Evaluations)       [optimization pack]
    ↓ next cycle
PolicyGateSuggestor (Evaluations → Constraints)  [policy]
```

Use case: **Sales operations** — segment customers, assign to reps,
enforce territory policies.

### Pattern: Forecast → Plan → Validate

```
forecasting (Seeds → Strategies)
    ↓ next cycle
inventory_replenishment (Strategies → Evaluations)  [optimization pack]
    ↓ next cycle
budget_allocation (Evaluations → Hypotheses)        [optimization pack]
```

Use case: **Supply chain** — forecast demand, plan replenishment,
allocate budget across warehouses.

### Pattern: Classify → Rank → Select

```
classification (Seeds → Strategies)
    ↓ next cycle
ranking (Strategies → Evaluations)
    ↓ next cycle
vendor_shortlist (Evaluations → Hypotheses)   [optimization pack]
```

Use case: **Procurement** — classify vendors by fit, rank by composite
criteria, shortlist top candidates.

### Pattern: Stats → Detect → Alert

```
descriptive_stats (Seeds → Strategies)
    ↓ next cycle
trend_detection (Strategies → Evaluations)
    ↓ next cycle
anomaly_detection (Evaluations → Hypotheses)
```

Use case: **Business intelligence** — summarize KPIs, detect shifts,
flag anomalies in context of historical trends.

### Pattern: Similarity → Segment → Act

```
similarity (Seeds → Strategies)
    ↓ next cycle
segmentation (Strategies → Evaluations)
    ↓ next cycle
pricing_guardrails (Evaluations → Constraints)  [optimization pack]
```

Use case: **Dynamic pricing** — find similar products, group into tiers,
enforce pricing guardrails per tier.

## Matching Use Cases to Packs

| I need to... | Pack | Feeds into... |
|---|---|---|
| Find unusual values in metrics | `anomaly_detection` | triage, alerting, capacity |
| Group customers/products/records | `segmentation` | routing, allocation, pricing |
| Rank candidates by multiple criteria | `ranking` | shortlisting, prioritization |
| Predict next period's values | `forecasting` | planning, replenishment, budgeting |
| Label records as yes/no | `classification` | routing, triage, filtering |
| Predict a numeric outcome | `regression` | pricing, scoring, planning |
| Find what's most alike | `similarity` | dedup, grouping, recommendations |
| Detect rising/falling/stable periods | `trend_detection` | alerting, rebalancing |
| Summarize data distributions | `descriptive_stats` | feeds any downstream pack |

## Invariant System

Each pack defines invariants that the gate evaluates before promoting results:

| Pack | Critical (blocks) | Advisory (escalates) |
|------|---|---|
| anomaly-detection | valid-statistics (std > 0) | anomaly-ratio (> 50% flagged) |
| segmentation | non-empty-clusters | balanced-clusters (< 10% expected) |
| ranking | valid-dimensions | score-separation (< 0.01 spread) |
| forecasting | finite-predictions | wide-intervals (4x growth) |
| classification | valid-probabilities | class-imbalance (> 90% one class) |
| regression | finite-values | zero-variance |
| similarity | valid-scores | low-discrimination (< 0.01 spread) |
| trend-detection | valid-segments (full coverage) | excessive-changepoints (> 50% of length) |
| descriptive-stats | finite-statistics | high-skew (|skewness| > 2) |

## Design Principles

1. **Not a generic ML library.** These are thin, opinionated wrappers that
   produce proposals for the convergence loop.

2. **Same Pack trait.** Analytics packs use the same `validate_inputs → solve →
   check_invariants → evaluate_gate` lifecycle as optimization packs.

3. **Pre-trained models are input.** For classification/regression, model weights
   arrive in the `ProblemSpec` inputs. Training happens outside the loop.

4. **Convergence participation.** The value is composition — an anomaly detector's
   output feeds a capacity planner in the next cycle. The Engine handles coordination.

5. **Simple algorithms by design.** Z-score, k-means, exponential smoothing,
   logistic sigmoid, linear regression. Not competing with scikit-learn — providing
   prepackaged suggestors that converge.

## File Layout

In the prism extension (`~/dev/extensions/prism`):

```
crates/analytics/src/
├── packs/
│   ├── mod.rs
│   ├── anomaly_detection/   (types.rs, solver.rs, mod.rs)
│   ├── classification/      (types.rs, solver.rs, mod.rs)
│   ├── descriptive_stats/   (types.rs, solver.rs, mod.rs)
│   ├── forecasting/         (types.rs, solver.rs, mod.rs)
│   ├── ranking/             (types.rs, solver.rs, mod.rs)
│   ├── regression/          (types.rs, solver.rs, mod.rs)
│   ├── segmentation/        (types.rs, solver.rs, mod.rs)
│   ├── similarity/          (types.rs, solver.rs, mod.rs)
│   └── trend_detection/     (types.rs, solver.rs, mod.rs)
├── suggestor.rs             (factory functions)
└── ...
```

## Example & Tests

- **Example:** `examples/analytics-packs/` (in prism) — runs all 9 packs, one at a time
- **Integration tests:** `crates/analytics/tests/pack_integration.rs` (in prism) — 9 tests,
  each proving a pack converges and produces correct output

See also: [[Formation Pattern]], [[Suggestor Contract]], [[API Surfaces]]
