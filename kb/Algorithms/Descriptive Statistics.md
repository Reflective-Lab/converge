---
source: llm
---

# Descriptive Statistics

**Complexity:** O(n log n) for sorting (median), O(n) for rest
**Converge module:** `converge_analytics::packs::descriptive_stats::DescriptiveStatsSolver`
**Reference:** Hand-computable. NIST StRD datasets for validation.

## Formulas

```
mean     = sum(x_i) / n
median   = middle value (or average of two middle values)
variance = sum((x_i - mean)^2) / n   (population variance)
std_dev  = sqrt(variance)
range    = max - min
```

## Worked Example

### Input

Values: [2, 4, 4, 4, 5, 5, 7, 9] (n = 8)

### Step 1: Mean

sum = 2 + 4 + 4 + 4 + 5 + 5 + 7 + 9 = 40
mean = 40 / 8 = **5.0**

### Step 2: Median

Sorted: [2, 4, 4, 4, 5, 5, 7, 9]. n=8 (even).
Median = (values[3] + values[4]) / 2 = (4 + 5) / 2 = **4.5**

### Step 3: Variance

| x_i | (x_i - 5)^2 |
|-----|-------------|
|  2  |      9      |
|  4  |      1      |
|  4  |      1      |
|  4  |      1      |
|  5  |      0      |
|  5  |      0      |
|  7  |      4      |
|  9  |     16      |

Sum of squares = 9+1+1+1+0+0+4+16 = 32
Variance = 32 / 8 = **4.0**

### Step 4: Standard deviation

std_dev = sqrt(4.0) = **2.0**

### Step 5: Range

min = 2, max = 9, range = 9 - 2 = **7**

### Result

| Statistic | Value |
|-----------|-------|
| Count     |   8   |
| Mean      |  5.0  |
| Median    |  4.5  |
| Variance  |  4.0  |
| Std Dev   |  2.0  |
| Min       |  2.0  |
| Max       |  9.0  |
| Range     |  7.0  |

## Why it matters for agents

**Business decision:** What does this data actually look like before we act on it. Descriptive statistics is the grounding step — it turns a raw signal into a trustworthy baseline. Without it, agents apply thresholds and comparisons to numbers they haven't characterized, producing confident-sounding outputs on top of meaningless inputs.

Typical decisions: is this dataset worth running a model on, what is the baseline KPI we are measuring deviation from, is this batch of sensor readings healthy or has the instrument drifted.

**Formation arc — data profiling before anomaly detection**

An analytics formation receives a new batch of weekly expense data. Before any anomaly detection or forecasting can be trusted, a profiling suggestor runs descriptive statistics and writes a baseline to Signals. The anomaly suggestor declares a dependency on the baseline.

```
Seeds ← "expense-batch:may-2026"
  values: [4200, 4350, 4100, 4500, 4250, 4400, 4300, 4450, 4200, 4380]

→ StatsSuggestor computes and writes:

Signals ← "stats:expense-batch-may-2026"
  mean:    4313.0
  median:  4325.0
  std_dev: 113.7
  min:     4100
  max:     4500
  range:   400
```

The anomaly suggestor reads these baseline stats and uses them to parameterize Z-score detection for the next batch. When the next batch arrives with a value of 6800, the z-score is (6800 - 4313) / 113.7 = 21.9 — unambiguously anomalous.

Without the baseline, the anomaly suggestor would have to compute statistics on-the-fly from every batch, unable to compare across time. The descriptive stats become a stable reference point that the formation anchors on.

**Why the math matters:** Mean alone misses skew. Stddev alone hides the range. Median separates from mean when there are outliers. A formation that only stores the mean as its baseline will set wrong thresholds on skewed data — the full profile is necessary for any downstream reasoning to be calibrated.

## Converge Validation

```
cargo test -p converge-analytics --test reference_validation descriptive_stats
```

All values confirmed.
