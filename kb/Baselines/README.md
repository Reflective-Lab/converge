---
name: Performance Baselines
description: Criterion benchmark results, trends, and regression detection
type: reference
source: human
---

# Performance Baselines

Performance baselines from Criterion benchmarks, tracked for regression detection and trend analysis.

## Format

### `latest-baseline.json`
Structured benchmark data from the most recent run:
```json
{
  "timestamp": "2026-04-17T12:34:56Z",
  "run_id": "20260417-123456",
  "benchmarks": {
    "engine_single_cycle": {
      "p50_us": 123.45,
      "p95_us": 145.67,
      "p99_us": 167.89,
      "mean_us": 125.00,
      "std_dev_us": 12.34
    }
  }
}
```

### `latest-summary.md`
Human-readable markdown table for quick review.

### `trends.csv`
Historical data for long-term trend analysis and regression detection:
```csv
date,run_id,benchmark,p50_us,p95_us,p99_us,mean_us,std_dev_us
2026-04-17,20260417-123456,engine_single_cycle,123.45,145.67,167.89,125.00,12.34
```

## Regression Detection

A regression is detected when:
- **p99 latency increases > 10%** from the previous baseline
- **Mean increases > 5%** from the previous baseline
- **StdDev increases > 20%** (indicates variability degradation)

## Benchmarks Tracked

| Benchmark | Purpose |
|-----------|---------|
| `engine_single_cycle` | Minimum latency: 1 suggestor, immediate convergence |
| `engine_multi_suggestor/1` | Single additional proposer |
| `engine_multi_suggestor/5` | 5 concurrent proposers |
| `engine_multi_suggestor/20` | 20 concurrent proposers |
| `engine_budget_pressure_near_ceiling` | Near-budget convergence behavior |
| `engine_large_context_1000_facts` | Scaling with large pre-seeded fact store |

## See Also

- `EXP-001.md` — Hypothesis and acceptance criteria for `engine_single_cycle` p99 < 1ms
- `.github/workflows/stability.yml` — Automated extraction on weekly schedule
- `ops/scripts/extract-criterion-baseline.py` — Extraction script
