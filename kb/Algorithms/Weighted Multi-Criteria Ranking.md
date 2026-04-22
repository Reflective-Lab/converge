---
source: llm
---

# Weighted Multi-Criteria Ranking

**Complexity:** O(n * c) where n = items, c = criteria
**Converge module:** `converge_analytics::packs::ranking::WeightedScoringSolver`
**Reference:** Standard MCDM (Multi-Criteria Decision Making). See Hwang & Yoon (1981), *Multiple Attribute Decision Making*.

## Algorithm Steps

1. **Min-max normalize** each criterion to [0, 1]:
   ```
   normalized = (x - min) / (max - min)
   ```
   If direction is "lower is better", invert: normalized = 1 - normalized.

2. **Weight and sum:**
   ```
   composite = sum(w_i * normalized_i)
   ```

3. **Rank** by composite score (descending).

## Worked Example

### Input

3 items, 2 criteria, weights = [0.7, 0.3], both higher-is-better.

| Item | Criterion 1 | Criterion 2 |
|------|------------|------------|
|  A   |    100     |     10     |
|  B   |     50     |     90     |
|  C   |     75     |     50     |

### Step 1: Normalize

**Criterion 1:** min=50, max=100
- A: (100-50)/(100-50) = 1.0
- B: (50-50)/(100-50) = 0.0
- C: (75-50)/(100-50) = 0.5

**Criterion 2:** min=10, max=90
- A: (10-10)/(90-10) = 0.0
- B: (90-10)/(90-10) = 1.0
- C: (50-10)/(90-10) = 0.5

### Step 2: Weighted sum

| Item | 0.7 * C1 | 0.3 * C2 | Composite |
|------|----------|----------|-----------|
|  A   |   0.70   |   0.00   |   **0.70**|
|  B   |   0.00   |   0.30   |   **0.30**|
|  C   |   0.35   |   0.15   |   **0.50**|

### Step 3: Rank

1. A (0.70)
2. C (0.50)
3. B (0.30)

### Interpretation

Item A wins because criterion 1 has 70% weight and A dominates on it. Despite B's strong showing on criterion 2, the lower weight (30%) keeps it in last place.

## Why it matters for agents

**Business decision:** Which option is best when no single criterion dominates. Most real decisions involve trade-offs — a cheaper vendor is slower, a faster candidate has less experience. Weighted ranking makes the trade-off explicit and auditable: you specify the weights once, and the algorithm produces a reproducible ranking that cannot be argued with on arithmetic grounds.

Typical decisions: vendor selection, job candidate shortlist, product feature prioritization, market entry country ranking.

**Formation arc — vendor selection**

A procurement formation evaluates 4 vendors across 3 criteria: price (lower is better, weight 0.5), delivery time (lower is better, weight 0.3), and quality score (higher is better, weight 0.2). The weights were set by policy and live in Constraints.

```
Constraints ← "vendor-weights:procurement-policy"
  {"price": 0.5, "delivery_days": 0.3, "quality": 0.2}

Signals ← "vendor-bids:rfp-2026-q2"
  [
    {"name": "VendorA", "price": 1200, "delivery_days": 5,  "quality": 90},
    {"name": "VendorB", "price": 900,  "delivery_days": 12, "quality": 75},
    {"name": "VendorC", "price": 1100, "delivery_days": 7,  "quality": 85},
    {"name": "VendorD", "price": 800,  "delivery_days": 20, "quality": 60},
  ]
```

A `RankingSuggestor` normalizes, weights, and scores:

```
Strategies ← "vendor-ranking:rfp-2026-q2"
  ranked:
    1. VendorC  (score: 0.72)   ← balanced on all three
    2. VendorA  (score: 0.61)
    3. VendorB  (score: 0.54)
    4. VendorD  (score: 0.29)   ← price wins but delivery is too slow
```

The top-ranked vendor is automatically surfaced to a contracting suggestor. If the top result's confidence is below a threshold (e.g., VendorC is only marginally ahead of VendorA), a human-in-the-loop gate is proposed before committing.

**Why the math matters:** Without normalization, raw numbers with different scales dominate. A price difference of $400 would swamp a quality difference of 15 points simply because dollars are larger numbers than quality scores. Min-max normalization puts every criterion on the same [0,1] scale so weights are the only thing that matters.

## Converge Validation

```
cargo test -p converge-analytics --test reference_validation ranking
```

Ranking order (A > C > B) and composite scores (0.70, 0.50, 0.30) confirmed.
