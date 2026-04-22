---
source: llm
---

# Z-Score Anomaly Detection

**Complexity:** O(n)
**Converge module:** `converge_analytics::packs::anomaly_detection::ZScoreSolver`
**Reference:** Standard statistical method. Any introductory statistics textbook.

## Formula

```
z_i = (x_i - mu) / sigma
```

Where mu = population mean, sigma = population standard deviation. Flag as anomaly if |z_i| > threshold.

## Worked Example

### Input

Values: [10, 10, 10, 10, 10, 10, 10, 10, 10, 100]
Threshold: 2.0

### Step 1: Compute mean

mu = (9 x 10 + 100) / 10 = 190 / 10 = **19.0**

### Step 2: Compute standard deviation

Variance = (9 x (10-19)^2 + (100-19)^2) / 10
         = (9 x 81 + 6561) / 10
         = (729 + 6561) / 10
         = 7290 / 10 = 729.0

sigma = sqrt(729) = **27.0**

### Step 3: Compute z-scores

| Value | z = (x - 19) / 27 | |z| > 2? |
|-------|--------------------|---------| 
|  10   | -9/27 = -0.333     | No      |
| 100   | 81/27 = 3.000      | **Yes** |

### Result

1 anomaly detected: index 9 (value=100, z=3.0).

## Converge Validation

```
cargo test -p converge-analytics --test reference_validation zscore
```

Mean=19.0, stddev=27.0, 1 anomaly at index 9 confirmed.
