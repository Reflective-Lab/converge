---
source: llm
---

# Simple Exponential Smoothing (SES)

**Complexity:** O(n)
**Converge module:** `converge_analytics::packs::forecasting::ExponentialSmoothingSolver`
**Reference:** Hyndman, R. J.; Athanasopoulos, G. (2021). *Forecasting: Principles and Practice*, 3rd ed. Chapter 8.

## Formula

```
level[0] = y[0]                              (initialize to first observation)
level[t] = alpha * y[t] + (1 - alpha) * level[t-1]   (for t >= 1)
forecast = level[n-1]                        (flat forecast from last level)
```

Where alpha in (0, 1) is the smoothing parameter:
- alpha near 1: heavy weight on recent observations (responsive, noisy)
- alpha near 0: heavy weight on history (smooth, slow to adapt)

## Worked Example

### Input

Values: [100, 110, 120], alpha = 0.5, horizon = 1

### Step-by-step

**t=0:** level[0] = y[0] = 100.0

**t=1:** level[1] = 0.5 * 110 + 0.5 * 100 = 55 + 50 = **105.0**

**t=2:** level[2] = 0.5 * 120 + 0.5 * 105 = 60 + 52.5 = **112.5**

### Forecast

1-step-ahead forecast = level[2] = **112.5**

### Intuition

The true next value in this linear trend would be 130. SES lags behind trends because it's designed for level data (no trend component). For trending data, use Holt's method (double exponential smoothing).

## Converge Validation

```
cargo test -p converge-analytics --test reference_validation exponential_smoothing
```

Forecast = 112.5 confirmed.
