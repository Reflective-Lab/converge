---
tags: [stack, analytics]
source: mixed
---
# Polars

Polars is used for data analytics and metrics computation within the LLM and analytics pipelines.

## Role in Converge

Polars is the foundation of the **prism** extension's analytics pipelines
(`~/dev/reflective/mosaic-extensions/prism`, formerly `converge-analytics`). Foundation no
longer ships Polars-backed pack code. The runway `converge-llm` bridge
module converts analysis dataframes into state injections for agent
prompts.

```rust
PolarsMetrics → MetricsBuilder → to_state_injection()
```

## Use Cases

- Metrics computation from convergence run data
- Temporal analysis (cost projections, trend detection)
- DataFrame operations over fact sets
- Analytical agent backends

## Why Polars

- Pure Rust — no Python dependency
- Arrow-native — interops with [[Stack/LanceDB]]
- Fast columnar operations — efficient for metrics over large fact sets
- Lazy evaluation — query optimization for complex analytics

See also: [[Stack/LanceDB]], [[Stack/Burn]], [[Architecture/Providers]]
