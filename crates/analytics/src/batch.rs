// Copyright 2024-2026 Reflective Labs

//! Batch feature extraction utilities for temporal anomaly detection.
//!
//! Provides high-level functions that abstract Polars internals, returning
//! plain Rust types. Consumers (spikes, applications) never see Polars
//! `DataFrame` or `LazyFrame` — only [`TemporalFeatures`] and [`FeatureVector`].

use anyhow::Result;
use polars::prelude::*;
use serde::{Deserialize, Serialize};

use crate::engine::FeatureVector;

/// Temporal features extracted for a single entity (user, device, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalFeatures {
    pub entity_id: String,
    pub event_count: u32,
    pub mean_delta_s: f64,
    pub min_delta_s: f64,
    pub std_delta_s: f64,
    pub burst_score: u32,
    pub type_entropy: f64,
    pub unique_categories: u32,
    pub night_ratio: f64,
}

/// Configuration for temporal feature extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalFeatureConfig {
    pub entity_column: String,
    pub timestamp_column: String,
    pub type_column: String,
    pub category_column: String,
    pub burst_threshold_seconds: i64,
}

impl Default for TemporalFeatureConfig {
    fn default() -> Self {
        Self {
            entity_column: "user_id".into(),
            timestamp_column: "timestamp".into(),
            type_column: "event_type".into(),
            category_column: "repo_id".into(),
            burst_threshold_seconds: 60,
        }
    }
}

/// Extract temporal features from a Parquet file.
///
/// Loads the Parquet file using Polars, groups events by entity, and computes
/// per-entity temporal features. Returns plain Rust types — no Polars dependency
/// leaks to the caller.
///
/// # Features computed
///
/// - `event_count` — total events
/// - `mean_delta_s` — average inter-event time (seconds)
/// - `min_delta_s` — minimum inter-event time
/// - `std_delta_s` — standard deviation of inter-event times
/// - `burst_score` — count of events with delta below threshold
/// - `type_entropy` — Shannon entropy of event type distribution
/// - `unique_categories` — number of distinct categories (repos, etc.)
/// - `night_ratio` — fraction of events during 00:00–06:00 UTC
pub fn extract_temporal_features(
    parquet_path: &str,
    config: &TemporalFeatureConfig,
) -> Result<Vec<TemporalFeatures>> {
    let ec = &config.entity_column;
    let tc = &config.timestamp_column;
    let tyc = &config.type_column;
    let cc = &config.category_column;

    // Load events from Parquet or CSV (detect by extension).
    let events = load_events_lazy(parquet_path)?;

    // Aggregates: count, unique categories.
    let aggregates = events
        .clone()
        .group_by([col(ec)])
        .agg([
            col(tc).count().alias("event_count"),
            col(cc).n_unique().alias("unique_categories"),
        ])
        .collect()?;

    // Temporal deltas: sort by entity+time, shift within groups, compute delta.
    let with_deltas = events
        .clone()
        .sort([ec, tc], Default::default())
        .with_columns([(col(tc) - col(tc).shift(lit(1)).over([col(ec)])).alias("delta_s")])
        .filter(col("delta_s").is_not_null())
        .group_by([col(ec)])
        .agg([
            col("delta_s").mean().alias("mean_delta_s"),
            col("delta_s").min().alias("min_delta_s"),
            col("delta_s").std(1).alias("std_delta_s"),
            col("delta_s")
                .lt(lit(config.burst_threshold_seconds))
                .sum()
                .alias("burst_score"),
        ])
        .collect()?;

    // Type entropy: -Σ p(t) * ln(p(t)).
    let entropy = compute_type_entropy_polars(&events, ec, tyc)?;

    // Night ratio: fraction of events during 00:00–06:00 UTC.
    let night = events
        .clone()
        .with_columns([((col(tc) % lit(86400)) / lit(3600)).alias("hour")])
        .group_by([col(ec)])
        .agg([col("hour")
            .lt(lit(6))
            .cast(DataType::Float64)
            .mean()
            .alias("night_ratio")])
        .collect()?;

    // Join all feature sets.
    let features = aggregates
        .lazy()
        .join(
            with_deltas.lazy(),
            [col(ec)],
            [col(ec)],
            JoinArgs::new(JoinType::Left),
        )
        .join(
            entropy.lazy(),
            [col(ec)],
            [col(ec)],
            JoinArgs::new(JoinType::Left),
        )
        .join(
            night.lazy(),
            [col(ec)],
            [col(ec)],
            JoinArgs::new(JoinType::Left),
        )
        .with_columns([
            col("mean_delta_s").fill_null(lit(0.0)),
            col("min_delta_s").fill_null(lit(0.0)),
            col("std_delta_s").fill_null(lit(0.0)),
            col("burst_score").fill_null(lit(0)),
            col("type_entropy").fill_null(lit(0.0)),
            col("night_ratio").fill_null(lit(0.0)),
        ])
        .collect()?;

    // Convert to Vec<TemporalFeatures>.
    dataframe_to_temporal_features(&features, ec)
}

/// Load events lazily from Parquet or CSV based on file extension.
fn load_events_lazy(path: &str) -> Result<LazyFrame> {
    if std::path::Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("csv"))
    {
        let pb = std::path::PathBuf::from(path);
        Ok(CsvReadOptions::default()
            .with_has_header(true)
            .try_into_reader_with_file_path(Some(pb))?
            .finish()?
            .lazy())
    } else {
        let pl_path = PlPath::from_str(path);
        Ok(LazyFrame::scan_parquet(pl_path, Default::default())?)
    }
}

/// Compute Shannon entropy of event type distribution per entity.
fn compute_type_entropy_polars(events: &LazyFrame, ec: &str, tyc: &str) -> Result<DataFrame> {
    let counts = events
        .clone()
        .group_by([col(ec), col(tyc)])
        .agg([col(ec).count().alias("type_count")])
        .collect()?;

    let totals = counts
        .clone()
        .lazy()
        .group_by([col(ec)])
        .agg([col("type_count").sum().alias("total_count")])
        .collect()?;

    let with_prob = counts
        .lazy()
        .join(
            totals.lazy(),
            [col(ec)],
            [col(ec)],
            JoinArgs::new(JoinType::Left),
        )
        .with_columns([(col("type_count").cast(DataType::Float64)
            / col("total_count").cast(DataType::Float64))
        .alias("prob")])
        .with_columns([
            (col("prob") * col("prob").log(lit(std::f64::consts::E)) * lit(-1.0))
                .alias("entropy_contrib"),
        ])
        .group_by([col(ec)])
        .agg([col("entropy_contrib").sum().alias("type_entropy")])
        .collect()?;

    Ok(with_prob)
}

/// Convert a joined feature DataFrame to `Vec<TemporalFeatures>`.
fn dataframe_to_temporal_features(df: &DataFrame, ec: &str) -> Result<Vec<TemporalFeatures>> {
    let ids = df.column(ec)?.str()?;
    let counts = df.column("event_count")?.cast(&DataType::UInt32)?;
    let counts = counts.u32()?;
    let mean_d = df.column("mean_delta_s")?.cast(&DataType::Float64)?;
    let mean_d = mean_d.f64()?;
    let min_d = df.column("min_delta_s")?.cast(&DataType::Float64)?;
    let min_d = min_d.f64()?;
    let std_d = df.column("std_delta_s")?.cast(&DataType::Float64)?;
    let std_d = std_d.f64()?;
    let burst = df.column("burst_score")?.cast(&DataType::UInt32)?;
    let burst = burst.u32()?;
    let entropy = df.column("type_entropy")?.cast(&DataType::Float64)?;
    let entropy = entropy.f64()?;
    let uniq = df.column("unique_categories")?.cast(&DataType::UInt32)?;
    let uniq = uniq.u32()?;
    let night = df.column("night_ratio")?.cast(&DataType::Float64)?;
    let night = night.f64()?;

    let mut result = Vec::with_capacity(df.height());
    for i in 0..df.height() {
        result.push(TemporalFeatures {
            entity_id: ids.get(i).unwrap_or("?").to_string(),
            event_count: counts.get(i).unwrap_or(0),
            mean_delta_s: mean_d.get(i).unwrap_or(0.0),
            min_delta_s: min_d.get(i).unwrap_or(0.0),
            std_delta_s: std_d.get(i).unwrap_or(0.0),
            burst_score: burst.get(i).unwrap_or(0),
            type_entropy: entropy.get(i).unwrap_or(0.0),
            unique_categories: uniq.get(i).unwrap_or(0),
            night_ratio: night.get(i).unwrap_or(0.0),
        });
    }

    Ok(result)
}

/// Compute z-scores for a set of values: `(x - mean) / std`.
///
/// Pure Rust — no Polars dependency.
pub fn z_scores(values: &[f64]) -> Vec<f64> {
    if values.is_empty() {
        return vec![];
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    let std = variance.sqrt().max(1e-10);
    values.iter().map(|x| (x - mean) / std).collect()
}

/// Convert [`TemporalFeatures`] to a [`FeatureVector`] suitable for ML inference.
///
/// Returns an [n_entities, 8] matrix with columns:
/// `[event_count, mean_delta_s, min_delta_s, std_delta_s, burst_score, type_entropy, unique_categories, night_ratio]`
pub fn temporal_to_feature_vector(features: &[TemporalFeatures]) -> Result<FeatureVector> {
    let n = features.len();
    let cols = 8;
    let mut data = Vec::with_capacity(n * cols);
    for f in features {
        data.push(f.event_count as f32);
        data.push(f.mean_delta_s as f32);
        data.push(f.min_delta_s as f32);
        data.push(f.std_delta_s as f32);
        data.push(f.burst_score as f32);
        data.push(f.type_entropy as f32);
        data.push(f.unique_categories as f32);
        data.push(f.night_ratio as f32);
    }
    FeatureVector::new(data, [n, cols])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn z_scores_centers_and_scales() {
        let vals = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let z = z_scores(&vals);
        // Mean of z-scores should be ~0.
        let mean: f64 = z.iter().sum::<f64>() / z.len() as f64;
        assert!(mean.abs() < 1e-10);
        // Std of z-scores should be ~1.
        let std: f64 = (z.iter().map(|x| x.powi(2)).sum::<f64>() / z.len() as f64).sqrt();
        assert!((std - 1.0).abs() < 1e-10);
    }

    #[test]
    fn z_scores_empty_returns_empty() {
        assert!(z_scores(&[]).is_empty());
    }

    #[test]
    fn temporal_to_feature_vector_shape() {
        let features = vec![
            TemporalFeatures {
                entity_id: "a".into(),
                event_count: 10,
                mean_delta_s: 100.0,
                min_delta_s: 5.0,
                std_delta_s: 50.0,
                burst_score: 3,
                type_entropy: 1.5,
                unique_categories: 5,
                night_ratio: 0.1,
            },
            TemporalFeatures {
                entity_id: "b".into(),
                event_count: 20,
                mean_delta_s: 200.0,
                min_delta_s: 10.0,
                std_delta_s: 80.0,
                burst_score: 0,
                type_entropy: 1.8,
                unique_categories: 10,
                night_ratio: 0.0,
            },
        ];
        let fv = temporal_to_feature_vector(&features).unwrap();
        assert_eq!(fv.rows(), 2);
        assert_eq!(fv.cols(), 8);
    }
}
