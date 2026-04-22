---
source: llm
---

# Linear Regression (Inference)

**Complexity:** O(n * d) where n = records, d = features
**Converge module:** `converge_analytics::packs::regression::LinearRegressionSolver`
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

## Converge Validation

```
cargo test -p converge-analytics --test reference_validation linear_regression
```

All predictions (6.0, 5.0, 10.0) and mean (7.0) confirmed.
