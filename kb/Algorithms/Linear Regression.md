---
source: llm
---

# Linear Regression (Inference)

**Complexity:** O(n * d) where n = records, d = features
**Converge module:** `prism::packs::regression::LinearRegressionSolver`
**Reference:** Trivially verifiable: y = w . x + b

## Formula

```
y = w_1 * x_1 + w_2 * x_2 + ... + w_d * x_d + b
```

The converge solver performs inference (prediction) given known weights and bias, not training.

## Worked Example

### Input

Model: y = 2*x1 + 3*x2 + 1

Weights: [2, 3], Bias: 1

| Record | x1 | x2 | Calculation          | y  |
|--------|----|----|----------------------|----|
|   1    |  1 |  1 | 2(1) + 3(1) + 1      |  6 |
|   2    |  2 |  0 | 2(2) + 3(0) + 1      |  5 |
|   3    |  0 |  3 | 2(0) + 3(3) + 1      | 10 |

### Result

| Record | Prediction |
|--------|-----------|
|   1    |    6.0    |
|   2    |    5.0    |
|   3    |   10.0    |

Mean prediction = (6 + 5 + 10) / 3 = **7.0**

## Why it matters for agents

**Business decision:** Given what we know about this situation, what number should we expect. Linear regression (used here for inference, not training) is the bridge between a trained model and an agent decision. The model was fitted offline; the agent applies it at formation time to score new situations.

Typical decisions: estimated deal value given company size and product tier, predicted project effort given scope indicators, expected churn score given usage metrics.

**Formation arc — deal scoring in a sales formation**

A sales formation needs to estimate the deal value for each inbound lead before routing it (Hungarian assignment) to a rep. A pre-trained revenue model with weights [deal_size_factor=2.0, company_tier_factor=3.0] and bias 1.0 is stored in Constraints. The formation applies inference to each lead.

```
Constraints ← "revenue-model:smb-segment"
  weights: [2.0, 3.0]
  bias: 1.0

Seeds ← "leads:inbound-2026-04"
  records:
    [("lead-001", x=[1, 2]),   ← small company, tier 2
     ("lead-002", x=[3, 1]),   ← larger company, tier 1
     ("lead-003", x=[2, 3])]   ← mid company, tier 3
```

A `ScoringsSuggestor` runs inference and writes:

```
Signals ← "deal-scores:inbound-2026-04"
  [("lead-001", score=9.0),   ← 2(1)+3(2)+1
   ("lead-002", score=10.0),  ← 2(3)+3(1)+1
   ("lead-003", score=14.0)]  ← 2(2)+3(3)+1
```

The scores become the value column in the `portfolio-request` (if budget-constrained) or the cost column in an `assignment-request` (if routing to reps). The formation uses the linear model to quantify what otherwise would be a qualitative gut call.

**Why the math matters:** Two leads that look similar to a rep can have a 5× difference in expected value once tenure, company size, and tier are properly weighted. The model captures patterns across thousands of historical deals that no individual rep can hold in their head.

## Converge Validation

```
cargo test -p prism --test reference_validation linear_regression
```

All predictions (6.0, 5.0, 10.0) and mean (7.0) confirmed.
