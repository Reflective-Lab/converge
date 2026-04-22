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

## Converge Validation

```
cargo test -p converge-analytics --test reference_validation descriptive_stats
```

All values confirmed.
