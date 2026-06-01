---
tags: [modules, analytics]
source: mixed
---
# Analytics Module

> **Extracted to extension on 2026-05-05.** Analytics now lives in the
> **prism** extension (`~/dev/reflective/mosaic-extensions/prism`, formerly `converge-analytics`).
> Foundation no longer ships Polars/Burn ML pipelines or analytics packs. See
> [[Architecture/Extension Topology]].

The prism extension provides ML pipeline stages as Suggestors. Each stage
participates in the convergence loop, enabling data-driven decision making
alongside reasoning agents.

## Available Suggestors (12)

| Agent | Purpose | Reads | Writes |
|-------|---------|-------|--------|
| DataValidation | Validates input data quality | Seeds | Constraints (if invalid) |
| FeatureEngineering | Computes features from raw data | Seeds | Hypotheses |
| FeatureAgent | Polars-based feature extraction | Seeds | Hypotheses |
| Dataset | Prepares training datasets | Hypotheses | Seeds (dataset ref) |
| ModelTraining | Trains ML models | Seeds (dataset) | Strategies |
| HyperparameterSearch | Optimizes model params | Seeds | Strategies |
| ModelEvaluation | Evaluates model quality | Strategies | Evaluations |
| ModelRegistry | Registers passing models | Evaluations | Seeds (model ref) |
| InferenceAgent | Runs inference on new data | Seeds (model + input) | Strategies |
| Monitoring | Tracks model drift/performance | Strategies | Constraints (if degraded) |
| Deployment | Deploys validated models | Evaluations | Seeds (deployment ref) |
| SampleInference | Demo inference for validation | Seeds | Strategies |

## Usage in a Formation

```rust
use prism::engine::FeatureAgent;

engine.register_suggestor(FeatureAgent::new(feature_config));
engine.register_suggestor(InferenceAgent::new(model_path));
```

## ML Pipeline as Convergence

The ML pipeline is naturally a convergence problem:
1. Data validation writes constraints if data is bad
2. Feature engineering writes hypotheses (computed features)
3. Training writes strategies (candidate models)
4. Evaluation writes evaluations (quality metrics)
5. If evaluation fails → training re-runs with different params
6. Fixed point = validated model ready for deployment

## Polars Integration

Analytics uses Polars for vectorized computation:
- DataFrame operations in `execute()`
- Feature extraction, aggregation, joins
- Zero-copy where possible

## Feature Flags

- `excel`: Enables Excel file ingestion via calamine

See also: [[Architecture/Suggestor Contract]], [[Architecture/Formation Pattern]]
