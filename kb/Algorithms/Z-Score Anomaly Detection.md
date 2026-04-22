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

## Why it matters for agents

**Business decision:** Is something wrong right now. Z-score detection is the simplest always-on sensor that converts a stream of numbers into a binary signal: normal or not. Because it is O(n) and stateless once mean and stddev are known, it can run every convergence cycle without cost.

Typical decisions: is this invoice amount anomalous, is this API latency spike worth escalating, has this KPI deviated beyond acceptable range this week.

**Formation arc — spend monitoring**

A finance formation watches weekly vendor spend. Each week's total spend is seeded as a signal. The anomaly suggestor reads the signal history and flags anything beyond 2.0 standard deviations.

```
Signals ← "spend-history:vendor-cloud-infra"
  content: [18500, 19200, 18900, 19100, 18700, 19000, 19300, 47000]
  ← last value is the spike
```

An `AnomalyDetectionSuggestor` computes mean=22712, stddev=9835, z(47000) = 2.47 → anomaly:

```
Diagnostic ← "anomaly:spend-vendor-cloud-infra"
  {"index": 7, "value": 47000, "z_score": 2.47, "threshold": 2.0}
```

This Diagnostic fact triggers downstream suggestors. A triage suggestor reads the diagnostic and proposes an escalation to Proposals — a human-in-the-loop gate before any automated response. The formation converges only when the anomaly is either resolved or acknowledged.

**Why the math matters:** An agent checking "is this number bigger than last week's?" misses gradual drift and over-fires on natural variance. Z-score anchors the threshold to the actual distribution of the data — as the baseline shifts, the threshold shifts with it automatically.

## Converge Validation

```
cargo test -p converge-analytics --test reference_validation zscore
```

Mean=19.0, stddev=27.0, 1 anomaly at index 9 confirmed.
