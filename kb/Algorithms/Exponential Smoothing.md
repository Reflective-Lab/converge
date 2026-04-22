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

## Why it matters for agents

**Business decision:** What will the next value be. Exponential smoothing is the baseline forecasting primitive — cheap enough to run on every cycle, interpretable enough to use as a confidence signal for downstream decisions. It does not require training data in the ML sense; it only needs the historical series.

Typical decisions: next-week demand forecast for inventory replenishment, next-quarter revenue projection for budget planning, next-month cloud spend estimate for cost control alerts.

**Formation arc — demand-driven flow optimization**

A supply chain formation forecasts next week's demand, then feeds that forecast as the `demand` field into a `FlowOptimizationSuggestor`. The two suggestors form a pipeline that converges: forecast first, route second.

```
Signals ← "demand-history:product-SKU-4421"
  weekly_units: [820, 850, 810, 870, 840, 890, 860]
  alpha: 0.3

→ ForecastingSuggestor computes:
  level[6] = 0.3*860 + 0.7*prev_level = ...
  forecast = 855 units

Strategies ← "forecast:product-SKU-4421-week43"
  predicted_demand: 855
  alpha: 0.3
  confidence: 0.85   ← derived from recent forecast error
```

A translation suggestor reads the forecast and seeds:

```
Seeds ← "flow-request:logistics-week43"
  demand: 855         ← from forecast
```

`FlowOptimizationSuggestor` runs and routes the forecasted demand through the supply network. If `fulfillment < 1.0`, the capacity gap is known before the week begins — not discovered on Friday when it is too late.

**Why the math matters:** A static demand assumption (last week's actual) misses trends and seasonal swings. SES weighted toward recent observations (alpha=0.3) adapts more slowly, producing smoother signals that don't over-react to one-off spikes — appropriate for operational decisions that commit resources days in advance.

## Converge Validation

```
cargo test -p converge-analytics --test reference_validation exponential_smoothing
```

Forecast = 112.5 confirmed.
