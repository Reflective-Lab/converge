// Copyright 2024-2026 Reflective Labs

use anyhow::{Result, anyhow};
use converge_core::{AgentEffect, ContextKey, ProposedFact, Suggestor};
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A fact content representing computed features.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct FeatureVector {
    pub data: Vec<f32>,
    pub shape: [usize; 2],
}

impl FeatureVector {
    pub fn new(data: Vec<f32>, shape: [usize; 2]) -> Result<Self> {
        let expected = shape
            .first()
            .and_then(|rows| shape.get(1).map(|cols| rows.saturating_mul(*cols)))
            .unwrap_or(0);
        if data.len() != expected {
            return Err(anyhow!(
                "feature data length {} does not match shape {:?}",
                data.len(),
                shape
            ));
        }
        Ok(Self { data, shape })
    }

    pub fn row(data: Vec<f32>) -> Self {
        let cols = data.len();
        Self {
            data,
            shape: [1, cols],
        }
    }

    pub fn rows(&self) -> usize {
        self.shape[0]
    }

    pub fn cols(&self) -> usize {
        self.shape[1]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeatureColumns {
    pub left: String,
    pub right: String,
}

#[derive(Clone, Debug)]
pub struct FeatureAgent {
    source_path: Option<PathBuf>,
    columns: Option<FeatureColumns>,
}

impl FeatureAgent {
    pub fn new(source_path: Option<PathBuf>) -> Self {
        Self {
            source_path,
            columns: None,
        }
    }

    pub fn with_columns(mut self, left: impl Into<String>, right: impl Into<String>) -> Self {
        self.columns = Some(FeatureColumns {
            left: left.into(),
            right: right.into(),
        });
        self
    }

    /// Internal Polars logic to compute features
    fn compute_features(&self) -> Result<FeatureVector> {
        let df = if let Some(path) = &self.source_path {
            load_dataframe(path)?
        } else {
            df! [
                "x1" => [1.0, 2.0, 3.0],
                "x2" => [4.0, 5.0, 6.0],
                "x3" => [7.0, 8.0, 9.0],
            ]?
        };
        compute_features_from_df(&df, self.columns.as_ref())
    }
}

#[async_trait::async_trait]
impl Suggestor for FeatureAgent {
    fn name(&self) -> &str {
        "FeatureAgent (Polars)"
    }

    fn dependencies(&self) -> &[ContextKey] {
        // Depends on Seeds to know WHAT to process
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        // Run if we have Seeds but haven't produced Proposals yet
        ctx.has(ContextKey::Seeds) && !ctx.has(ContextKey::Proposals)
    }

    async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
        // 1. Compute features using Polars
        let features = match self.compute_features() {
            Ok(f) => f,
            Err(e) => {
                return AgentEffect::with_proposal(ProposedFact::new(
                    ContextKey::Diagnostic,
                    "feature-agent-error",
                    e.to_string(),
                    self.name(),
                ));
            }
        };

        // 2. Serialize to Fact content
        let content = serde_json::to_string(&features).unwrap_or_default();

        // 3. Propose the features
        let proposal = ProposedFact {
            key: ContextKey::Proposals,
            id: "features-001".into(),
            content,
            confidence: 1.0, // Deterministic computation
            provenance: "polars-engine".into(),
        };

        // Note: In a real agent, we might emit a Fact directly if trusted, or a ProposedFact.
        // converge_core usually requires TryFrom implementation or specific flow.
        // For simplicity, we assume we can emit effects.
        // Wait, AgentEffect::with_proposal?
        // Let's check AgentEffect definition.

        // Use the constructor for single proposal
        AgentEffect::with_proposal(proposal)
    }
}

fn compute_features_from_df(
    df: &DataFrame,
    columns: Option<&FeatureColumns>,
) -> Result<FeatureVector> {
    let (left, right) = if let Some(columns) = columns {
        let left = df
            .column(&columns.left)
            .map_err(|_| anyhow!("missing column {}", columns.left))?;
        let right = df
            .column(&columns.right)
            .map_err(|_| anyhow!("missing column {}", columns.right))?;
        (left.clone(), right.clone())
    } else {
        let mut numeric = df
            .get_columns()
            .iter()
            .filter(|series| is_numeric_dtype(series.dtype()))
            .cloned()
            .collect::<Vec<_>>();
        if numeric.len() < 2 {
            return Err(anyhow!("need at least two numeric columns"));
        }
        (numeric.remove(0), numeric.remove(0))
    };

    if left.len() == 0 || right.len() == 0 {
        return Err(anyhow!("input data is empty"));
    }

    let left = left.cast(&DataType::Float32)?;
    let right = right.cast(&DataType::Float32)?;

    let left_val = left
        .f32()?
        .get(0)
        .ok_or_else(|| anyhow!("missing left value"))?;
    let right_val = right
        .f32()?
        .get(0)
        .ok_or_else(|| anyhow!("missing right value"))?;

    let interaction = left_val * right_val;
    Ok(FeatureVector::row(vec![left_val, right_val, interaction]))
}

fn load_dataframe(path: &Path) -> Result<DataFrame> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow!("path is not valid utf-8: {}", path.display()))?;

    match extension.as_str() {
        "parquet" => {
            let pl_path = PlPath::from_str(path_str);
            Ok(LazyFrame::scan_parquet(pl_path, Default::default())?.collect()?)
        }
        "csv" => Ok(CsvReadOptions::default()
            .with_has_header(true)
            .try_into_reader_with_file_path(Some(path.to_path_buf()))?
            .finish()?),
        _ => Err(anyhow!(
            "unsupported data format for path {} (expected .csv or .parquet)",
            path.display()
        )),
    }
}

fn is_numeric_dtype(dtype: &DataType) -> bool {
    matches!(
        dtype,
        DataType::Int8
            | DataType::Int16
            | DataType::Int32
            | DataType::Int64
            | DataType::UInt8
            | DataType::UInt16
            | DataType::UInt32
            | DataType::UInt64
            | DataType::Float32
            | DataType::Float64
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashMap;
    use std::fs;
    use std::hint::black_box;
    use std::time::Instant;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn feature_vector_validates_shape() {
        let ok = FeatureVector::new(vec![1.0, 2.0], [1, 2]).unwrap();
        assert_eq!(ok.rows(), 1);
        assert_eq!(ok.cols(), 2);
        assert!(FeatureVector::new(vec![1.0], [1, 2]).is_err());
    }

    #[test]
    fn feature_vector_new_multi_row() {
        let fv = FeatureVector::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], [2, 3]).unwrap();
        assert_eq!(fv.rows(), 2);
        assert_eq!(fv.cols(), 3);
        assert_eq!(fv.data.len(), 6);
    }

    #[test]
    fn feature_vector_new_rejects_mismatched_length() {
        assert!(FeatureVector::new(vec![1.0, 2.0, 3.0], [2, 2]).is_err());
        assert!(FeatureVector::new(vec![], [1, 1]).is_err());
        assert!(FeatureVector::new(vec![1.0], [0, 1]).is_err());
    }

    #[test]
    fn feature_vector_new_empty() {
        let fv = FeatureVector::new(vec![], [0, 0]).unwrap();
        assert_eq!(fv.rows(), 0);
        assert_eq!(fv.cols(), 0);
        assert!(fv.data.is_empty());
    }

    #[test]
    fn feature_vector_new_zero_cols() {
        let fv = FeatureVector::new(vec![], [5, 0]).unwrap();
        assert_eq!(fv.rows(), 5);
        assert_eq!(fv.cols(), 0);
    }

    #[test]
    fn feature_vector_row_creates_single_row() {
        let fv = FeatureVector::row(vec![10.0, 20.0, 30.0]);
        assert_eq!(fv.rows(), 1);
        assert_eq!(fv.cols(), 3);
        assert_eq!(fv.data, vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn feature_vector_row_empty() {
        let fv = FeatureVector::row(vec![]);
        assert_eq!(fv.rows(), 1);
        assert_eq!(fv.cols(), 0);
        assert!(fv.data.is_empty());
    }

    #[test]
    fn feature_vector_row_single_element() {
        let fv = FeatureVector::row(vec![42.0]);
        assert_eq!(fv.rows(), 1);
        assert_eq!(fv.cols(), 1);
        assert_eq!(fv.data, vec![42.0]);
    }

    #[test]
    fn feature_columns_construction() {
        let fc = FeatureColumns {
            left: "price".to_string(),
            right: "quantity".to_string(),
        };
        assert_eq!(fc.left, "price");
        assert_eq!(fc.right, "quantity");
    }

    #[test]
    fn feature_columns_roundtrip_serde() {
        let fc = FeatureColumns {
            left: "a".to_string(),
            right: "b".to_string(),
        };
        let json = serde_json::to_string(&fc).unwrap();
        let deserialized: FeatureColumns = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.left, "a");
        assert_eq!(deserialized.right, "b");
    }

    #[test]
    fn feature_vector_roundtrip_serde() {
        let fv = FeatureVector::new(vec![1.0, 2.0, 3.0, 4.0], [2, 2]).unwrap();
        let json = serde_json::to_string(&fv).unwrap();
        let deserialized: FeatureVector = serde_json::from_str(&json).unwrap();
        assert_eq!(fv, deserialized);
    }

    #[test]
    fn feature_agent_new_without_columns() {
        let agent = FeatureAgent::new(None);
        assert!(agent.source_path.is_none());
        assert!(agent.columns.is_none());
    }

    #[test]
    fn feature_agent_with_columns() {
        let agent = FeatureAgent::new(None).with_columns("x", "y");
        let cols = agent.columns.unwrap();
        assert_eq!(cols.left, "x");
        assert_eq!(cols.right, "y");
    }

    #[test]
    fn feature_agent_with_source_path() {
        let agent = FeatureAgent::new(Some(PathBuf::from("/tmp/data.csv")));
        assert_eq!(agent.source_path.unwrap(), PathBuf::from("/tmp/data.csv"));
    }

    #[test]
    fn is_numeric_dtype_covers_all_numeric_types() {
        let numeric = [
            DataType::Int8,
            DataType::Int16,
            DataType::Int32,
            DataType::Int64,
            DataType::UInt8,
            DataType::UInt16,
            DataType::UInt32,
            DataType::UInt64,
            DataType::Float32,
            DataType::Float64,
        ];
        for dt in &numeric {
            assert!(is_numeric_dtype(dt), "{dt:?} should be numeric");
        }
    }

    #[test]
    fn is_numeric_dtype_rejects_non_numeric() {
        assert!(!is_numeric_dtype(&DataType::String));
        assert!(!is_numeric_dtype(&DataType::Boolean));
        assert!(!is_numeric_dtype(&DataType::Date));
    }

    #[test]
    fn compute_features_rejects_empty_dataframe() {
        let df = df![
            "a" => Vec::<f32>::new(),
            "b" => Vec::<f32>::new(),
        ]
        .unwrap();
        let cols = FeatureColumns {
            left: "a".into(),
            right: "b".into(),
        };
        assert!(compute_features_from_df(&df, Some(&cols)).is_err());
    }

    #[test]
    fn compute_features_rejects_missing_column() {
        let df = df!["a" => [1.0f32]].unwrap();
        let cols = FeatureColumns {
            left: "a".into(),
            right: "missing".into(),
        };
        assert!(compute_features_from_df(&df, Some(&cols)).is_err());
    }

    #[test]
    fn compute_features_rejects_insufficient_numeric_columns() {
        let df = df!["text" => ["a", "b"]].unwrap();
        assert!(compute_features_from_df(&df, None).is_err());
    }

    proptest! {
        #[test]
        fn feature_vector_shape_invariant(
            rows in 0usize..50,
            cols in 0usize..50,
        ) {
            let len = rows.saturating_mul(cols);
            let data = vec![0.0f32; len];
            let fv = FeatureVector::new(data, [rows, cols]).unwrap();
            prop_assert_eq!(fv.rows() * fv.cols(), fv.data.len());
        }
    }

    #[test]
    fn compute_features_from_df_uses_named_columns() {
        let df = df![
            "a" => [2.0f32, 3.0],
            "b" => [4.0f32, 5.0],
        ]
        .unwrap();

        let columns = FeatureColumns {
            left: "a".into(),
            right: "b".into(),
        };
        let features = compute_features_from_df(&df, Some(&columns)).unwrap();
        assert_eq!(features.data, vec![2.0, 4.0, 8.0]);
        assert_eq!(features.shape, [1, 3]);
    }

    #[test]
    fn compute_features_from_df_falls_back_to_first_numeric_columns() {
        let df = df![
            "text" => ["x", "y"],
            "a" => [1.5f32, 2.5],
            "b" => [3.0f32, 4.0],
        ]
        .unwrap();

        let features = compute_features_from_df(&df, None).unwrap();
        assert_eq!(features.data, vec![1.5, 3.0, 4.5]);
    }

    #[test]
    fn compute_features_handles_large_dataset() {
        let rows = 10_000;
        let left: Vec<f32> = (0..rows).map(|i| i as f32).collect();
        let right: Vec<f32> = (0..rows).map(|i| (i as f32) + 1.0).collect();
        let df = df![
            "left" => left,
            "right" => right,
        ]
        .unwrap();

        let columns = FeatureColumns {
            left: "left".into(),
            right: "right".into(),
        };
        let features = compute_features_from_df(&df, Some(&columns)).unwrap();
        assert_eq!(features.data, vec![0.0, 1.0, 0.0]);
    }

    #[test]
    fn load_dataframe_reads_csv() {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("converge_analytics_{nanos}.csv"));

        let contents = "left,right\n2.0,4.0\n3.0,5.0\n";
        fs::write(&path, contents).unwrap();

        let df = load_dataframe(&path).unwrap();
        assert_eq!(df.height(), 2);
        assert_eq!(df.width(), 2);
    }

    proptest! {
        #[test]
        fn compute_features_matches_first_row(
            left in proptest::collection::vec(prop::num::f32::NORMAL, 1..50),
            right in proptest::collection::vec(prop::num::f32::NORMAL, 1..50),
        ) {
            let len = left.len().min(right.len());
            let df = df![
                "left" => left[..len].to_vec(),
                "right" => right[..len].to_vec(),
            ]
            .unwrap();

            let columns = FeatureColumns {
                left: "left".into(),
                right: "right".into(),
            };
            let features = compute_features_from_df(&df, Some(&columns)).unwrap();
            let expected_left = left[0];
            let expected_right = right[0];
            prop_assert_eq!(features.data, vec![expected_left, expected_right, expected_left * expected_right]);
        }
    }

    #[test]
    fn polars_vectorized_dot_product_matches_naive() {
        let rows = 50_000;
        let left: Vec<f32> = (0..rows).map(|i| (i % 100) as f32).collect();
        let right: Vec<f32> = (0..rows).map(|i| ((i + 3) % 100) as f32).collect();
        let df = df![
            "left" => left.clone(),
            "right" => right.clone(),
        ]
        .unwrap();

        let product = (df.column("left").unwrap() * df.column("right").unwrap()).unwrap();
        let polars_sum = product
            .as_materialized_series()
            .cast(&DataType::Float64)
            .unwrap()
            .f64()
            .unwrap()
            .sum()
            .unwrap_or(0.0);

        let mut naive_sum = 0.0f64;
        for (l, r) in left.iter().zip(right.iter()) {
            naive_sum += (*l as f64) * (*r as f64);
        }

        assert!((polars_sum - naive_sum).abs() < 1e-6);
    }

    #[test]
    fn polars_groupby_sum_matches_naive() {
        let rows = 10_000;
        let keys: Vec<&str> = (0..rows)
            .map(|i| {
                if i % 3 == 0 {
                    "alpha"
                } else if i % 3 == 1 {
                    "beta"
                } else {
                    "gamma"
                }
            })
            .collect();
        let values: Vec<f32> = (0..rows).map(|i| (i % 7) as f32).collect();
        let df = df![
            "key" => keys.clone(),
            "value" => values.clone(),
        ]
        .unwrap();

        let grouped = df
            .lazy()
            .group_by([col("key")])
            .agg([col("value").sum().alias("value_sum")])
            .collect()
            .unwrap();
        let keys_series = grouped.column("key").unwrap().str().unwrap();
        let sums_series = grouped.column("value_sum").unwrap().f32().unwrap();

        let mut naive = HashMap::<&str, f32>::new();
        for (key, value) in keys.iter().zip(values.iter()) {
            *naive.entry(*key).or_insert(0.0) += value;
        }

        for idx in 0..grouped.height() {
            if let Some(key) = keys_series.get(idx) {
                let polars_value = sums_series.get(idx).unwrap_or(0.0);
                let naive_value = naive.get(key).copied().unwrap_or(0.0);
                assert!((polars_value - naive_value).abs() < 1e-3);
            }
        }
    }

    #[test]
    #[ignore]
    fn polars_vectorized_dot_product_is_fast() {
        let rows = 300_000;
        let left: Vec<f32> = (0..rows).map(|i| (i % 100) as f32).collect();
        let right: Vec<f32> = (0..rows).map(|i| ((i + 5) % 100) as f32).collect();

        let df = df![
            "left" => left.clone(),
            "right" => right.clone(),
        ]
        .unwrap();

        let polars_start = Instant::now();
        let product = (df.column("left").unwrap() * df.column("right").unwrap()).unwrap();
        let polars_sum = product
            .as_materialized_series()
            .f32()
            .unwrap()
            .sum()
            .unwrap_or(0.0);
        let polars_elapsed = polars_start.elapsed();
        black_box(polars_sum);

        let naive_start = Instant::now();
        let mut naive_sum = 0.0f32;
        for (l, r) in left.iter().zip(right.iter()) {
            naive_sum += l * r;
        }
        let naive_elapsed = naive_start.elapsed();
        black_box(naive_sum);

        println!(
            "polars dot product: {:?}, naive loop: {:?}",
            polars_elapsed, naive_elapsed
        );

        assert!(polars_elapsed <= naive_elapsed * 20);
    }

    #[test]
    #[ignore]
    fn polars_groupby_is_fast() {
        let rows = 200_000;
        let keys: Vec<&str> = (0..rows)
            .map(|i| {
                if i % 4 == 0 {
                    "alpha"
                } else if i % 4 == 1 {
                    "beta"
                } else if i % 4 == 2 {
                    "gamma"
                } else {
                    "delta"
                }
            })
            .collect();
        let values: Vec<f32> = (0..rows).map(|i| (i % 9) as f32).collect();
        let df = df![
            "key" => keys.clone(),
            "value" => values.clone(),
        ]
        .unwrap();

        let polars_start = Instant::now();
        let grouped = df
            .lazy()
            .group_by([col("key")])
            .agg([col("value").sum().alias("value_sum")])
            .collect()
            .unwrap();
        let polars_elapsed = polars_start.elapsed();
        black_box(grouped.height());

        let naive_start = Instant::now();
        let mut naive = HashMap::<&str, f32>::new();
        for (key, value) in keys.iter().zip(values.iter()) {
            *naive.entry(*key).or_insert(0.0) += value;
        }
        let naive_elapsed = naive_start.elapsed();
        black_box(naive.len());

        println!(
            "polars groupby: {:?}, naive hashmap: {:?}",
            polars_elapsed, naive_elapsed
        );

        assert!(polars_elapsed <= naive_elapsed * 20);
    }
}
