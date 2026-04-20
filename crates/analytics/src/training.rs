// Copyright 2024-2026 Reflective Labs

use anyhow::{Context as _, Result, anyhow};
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};

const DATASET_URL: &str = "https://huggingface.co/datasets/gvlassis/california_housing/resolve/refs%2Fconvert%2Fparquet/default/train/0000.parquet";
const TARGET_COLUMN: &str = "median_house_value";

fn proposal(
    provenance: &str,
    key: ContextKey,
    id: impl Into<String>,
    content: impl Into<String>,
) -> ProposedFact {
    ProposedFact::new(key, id, content, provenance)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrainingPlan {
    pub iteration: usize,
    pub max_rows: usize,
    pub train_fraction: f64,
    pub val_fraction: f64,
    pub infer_fraction: f64,
    pub quality_threshold: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatasetSplit {
    pub source_path: String,
    pub train_path: String,
    pub val_path: String,
    pub infer_path: String,
    pub total_rows: usize,
    pub max_rows: usize,
    pub train_rows: usize,
    pub val_rows: usize,
    pub infer_rows: usize,
    pub iteration: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BaselineModel {
    pub target_column: String,
    pub mean: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelMetadata {
    pub model_path: String,
    pub target_column: String,
    pub train_rows: usize,
    pub baseline_mean: f64,
    pub iteration: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvaluationReport {
    pub model_path: String,
    pub metric: String,
    pub value: f64,
    pub mean_abs_target: f64,
    pub success_ratio: f64,
    pub val_rows: usize,
    pub iteration: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InferenceSample {
    pub model_path: String,
    pub target_column: String,
    pub rows: usize,
    pub predictions: Vec<f64>,
    pub actuals: Vec<f64>,
    pub iteration: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataQualityReport {
    pub kind: String,
    pub iteration: usize,
    pub source_path: String,
    pub rows_checked: usize,
    pub missingness: HashMap<String, f64>,
    pub numeric_means: HashMap<String, f64>,
    pub outlier_counts: HashMap<String, usize>,
    pub drift_score: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeatureInteraction {
    pub name: String,
    pub left: String,
    pub right: String,
    pub op: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeatureSpec {
    pub kind: String,
    pub iteration: usize,
    pub target_column: String,
    pub numeric_features: Vec<String>,
    pub categorical_features: Vec<String>,
    pub normalization: String,
    pub interactions: Vec<FeatureInteraction>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HyperparameterSearchPlan {
    pub kind: String,
    pub iteration: usize,
    pub max_trials: usize,
    pub early_stopping: bool,
    pub params: HashMap<String, Vec<f64>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HyperparameterSearchResult {
    pub kind: String,
    pub iteration: usize,
    pub best_params: HashMap<String, f64>,
    pub score: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelRegistryRecord {
    pub kind: String,
    pub iteration: usize,
    pub model_path: String,
    pub metrics: HashMap<String, f64>,
    pub provenance: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonitoringReport {
    pub kind: String,
    pub iteration: usize,
    pub metric: String,
    pub value: f64,
    pub baseline: f64,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeploymentDecision {
    pub kind: String,
    pub iteration: usize,
    pub action: String,
    pub reason: String,
    pub retrain: bool,
}

#[derive(Debug)]
pub struct DatasetAgent {
    data_dir: PathBuf,
}

impl DatasetAgent {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn dataset_path(&self) -> PathBuf {
        self.data_dir.join("california_housing_train.parquet")
    }

    fn split_paths(&self) -> (PathBuf, PathBuf, PathBuf) {
        (
            self.data_dir.join("train.parquet"),
            self.data_dir.join("val.parquet"),
            self.data_dir.join("infer.parquet"),
        )
    }
}

#[derive(Debug, Default)]
pub struct DataValidationAgent;

impl DataValidationAgent {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Suggestor for DataValidationAgent {
    fn name(&self) -> &str {
        "DataValidationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Signals)
            && match read_latest_split_from_ctx(ctx) {
                Ok(split) => !has_data_quality_for_iteration(ctx, split.iteration),
                Err(_) => false,
            }
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let split = match read_latest_split_from_ctx(ctx) {
            Ok(split) => split,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "data-validation-error",
                    err.to_string(),
                ));
            }
        };

        let df = match load_dataframe(Path::new(&split.train_path)) {
            Ok(df) => df,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "data-validation-error",
                    err.to_string(),
                ));
            }
        };

        let rows = df.height();
        let mut missingness = HashMap::new();
        let mut numeric_means = HashMap::new();
        let mut outlier_counts = HashMap::new();

        for series in df.get_columns() {
            let name = series.name().to_string();
            let null_ratio = if rows > 0 {
                series.null_count() as f64 / rows as f64
            } else {
                0.0
            };
            missingness.insert(name.clone(), null_ratio);

            if is_numeric_dtype(series.dtype()) {
                if let Ok((mean, _std, outliers)) =
                    compute_numeric_stats(series.as_materialized_series())
                {
                    numeric_means.insert(name.clone(), mean);
                    outlier_counts.insert(name, outliers);
                }
            }
        }

        let drift_score = drift_score_from_ctx(ctx, split.iteration, &numeric_means);

        let report = DataQualityReport {
            kind: "data_quality".to_string(),
            iteration: split.iteration,
            source_path: split.train_path.clone(),
            rows_checked: rows,
            missingness,
            numeric_means,
            outlier_counts,
            drift_score,
        };

        let content = serde_json::to_string(&report).unwrap_or_default();
        AgentEffect::with_proposal(proposal(
            self.name(),
            ContextKey::Signals,
            format!("data-quality-{}", split.iteration),
            content,
        ))
    }
}

#[derive(Debug, Default)]
pub struct FeatureEngineeringAgent;

impl FeatureEngineeringAgent {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Suggestor for FeatureEngineeringAgent {
    fn name(&self) -> &str {
        "FeatureEngineeringAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Signals)
            && match read_latest_split_from_ctx(ctx) {
                Ok(split) => !has_feature_spec_for_iteration(ctx, split.iteration),
                Err(_) => false,
            }
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let split = match read_latest_split_from_ctx(ctx) {
            Ok(split) => split,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "feature-engineering-error",
                    err.to_string(),
                ));
            }
        };

        let df = match load_dataframe(Path::new(&split.train_path)) {
            Ok(df) => df,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "feature-engineering-error",
                    err.to_string(),
                ));
            }
        };

        let (target_column, _) = match select_target_column(&df) {
            Ok(value) => value,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "feature-engineering-error",
                    err.to_string(),
                ));
            }
        };

        let (numeric_features, categorical_features) = split_feature_columns(&df, &target_column);

        let mut interactions = Vec::new();
        if numeric_features.len() >= 2 {
            interactions.push(FeatureInteraction {
                name: format!("{}_x_{}", numeric_features[0], numeric_features[1]),
                left: numeric_features[0].clone(),
                right: numeric_features[1].clone(),
                op: "multiply".to_string(),
            });
        }

        let spec = FeatureSpec {
            kind: "feature_spec".to_string(),
            iteration: split.iteration,
            target_column,
            numeric_features,
            categorical_features,
            normalization: "standardize".to_string(),
            interactions,
        };

        let content = serde_json::to_string(&spec).unwrap_or_default();
        AgentEffect::with_proposal(proposal(
            self.name(),
            ContextKey::Constraints,
            format!("feature-spec-{}", split.iteration),
            content,
        ))
    }
}

#[derive(Debug)]
pub struct HyperparameterSearchAgent {
    max_trials: usize,
}

impl HyperparameterSearchAgent {
    pub fn new(max_trials: usize) -> Self {
        Self { max_trials }
    }
}

#[async_trait::async_trait]
impl Suggestor for HyperparameterSearchAgent {
    fn name(&self) -> &str {
        "HyperparameterSearchAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Constraints, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Signals)
            && match read_latest_split_from_ctx(ctx) {
                Ok(split) => !has_hyperparam_result_for_iteration(ctx, split.iteration),
                Err(_) => false,
            }
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let split = match read_latest_split_from_ctx(ctx) {
            Ok(split) => split,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "hyperparam-search-error",
                    err.to_string(),
                ));
            }
        };

        let training_plan = read_latest_plan_from_ctx(ctx).unwrap_or(TrainingPlan {
            iteration: split.iteration,
            max_rows: split.max_rows,
            train_fraction: 0.8,
            val_fraction: 0.15,
            infer_fraction: 0.05,
            quality_threshold: 0.75,
        });

        let mut params = HashMap::new();
        params.insert("learning_rate".to_string(), vec![0.001, 0.01, 0.1]);
        params.insert("hidden_size".to_string(), vec![8.0, 16.0, 32.0]);

        let plan = HyperparameterSearchPlan {
            kind: "hyperparam_plan".to_string(),
            iteration: split.iteration,
            max_trials: self.max_trials,
            early_stopping: true,
            params,
        };

        let mut best_params = HashMap::new();
        best_params.insert("learning_rate".to_string(), 0.01);
        best_params.insert("hidden_size".to_string(), 16.0);
        let score = (1.0 - training_plan.quality_threshold) * plan.max_trials as f64
            / plan.iteration.max(1) as f64;
        let result = HyperparameterSearchResult {
            kind: "hyperparam_result".to_string(),
            iteration: split.iteration,
            best_params,
            score,
        };

        let plan_content = serde_json::to_string(&plan).unwrap_or_default();
        let result_content = serde_json::to_string(&result).unwrap_or_default();

        let mut effect = AgentEffect::empty();
        effect.proposals.push(proposal(
            self.name(),
            ContextKey::Constraints,
            format!("hyperparam-plan-{}", split.iteration),
            plan_content,
        ));
        effect.proposals.push(proposal(
            self.name(),
            ContextKey::Evaluations,
            format!("hyperparam-result-{}", split.iteration),
            result_content,
        ));
        effect
    }
}

#[async_trait::async_trait]
impl Suggestor for DatasetAgent {
    fn name(&self) -> &str {
        "DatasetAgent (HuggingFace)"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        if !ctx.has(ContextKey::Seeds) {
            return false;
        }

        let plan = read_latest_plan_from_ctx(ctx);
        if let Some(plan) = plan {
            return !has_split_for_iteration(ctx, plan.iteration);
        }

        !ctx.has(ContextKey::Signals)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        if let Err(err) = create_dir_all(&self.data_dir) {
            return AgentEffect::with_proposal(proposal(
                self.name(),
                ContextKey::Diagnostic,
                "dataset-agent-error",
                err.to_string(),
            ));
        }

        let dataset_path = self.dataset_path();
        if let Err(err) = download_dataset_if_missing(&dataset_path) {
            return AgentEffect::with_proposal(proposal(
                self.name(),
                ContextKey::Diagnostic,
                "dataset-agent-error",
                err.to_string(),
            ));
        }

        let df = match load_dataframe(&dataset_path) {
            Ok(df) => df,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "dataset-agent-error",
                    err.to_string(),
                ));
            }
        };

        let total_rows = df.height();
        if total_rows < 10 {
            return AgentEffect::with_proposal(proposal(
                self.name(),
                ContextKey::Diagnostic,
                "dataset-agent-error",
                "dataset too small for splitting",
            ));
        }

        let plan = read_latest_plan_from_ctx(ctx).unwrap_or(TrainingPlan {
            iteration: 1,
            max_rows: total_rows,
            train_fraction: 0.8,
            val_fraction: 0.15,
            infer_fraction: 0.05,
            quality_threshold: 0.75,
        });

        let max_rows = plan.max_rows.min(total_rows).max(10);
        let df = df.slice(0, max_rows);

        let mut train_rows = ((max_rows as f64) * plan.train_fraction).floor() as usize;
        let mut val_rows = ((max_rows as f64) * plan.val_fraction).floor() as usize;
        let mut infer_rows = max_rows.saturating_sub(train_rows + val_rows);
        if infer_rows == 0 {
            if val_rows > 1 {
                val_rows -= 1;
            } else if train_rows > 1 {
                train_rows -= 1;
            }
            infer_rows = max_rows.saturating_sub(train_rows + val_rows).max(1);
        }

        let (train_path, val_path, infer_path) = self.split_paths();
        let train_df = df.slice(0, train_rows);
        let val_df = df.slice(train_rows as i64, val_rows);
        let infer_df = df.slice((train_rows + val_rows) as i64, infer_rows);

        if let Err(err) = write_parquet(&train_df, &train_path)
            .and_then(|_| write_parquet(&val_df, &val_path))
            .and_then(|_| write_parquet(&infer_df, &infer_path))
        {
            return AgentEffect::with_proposal(proposal(
                self.name(),
                ContextKey::Diagnostic,
                "dataset-agent-error",
                err.to_string(),
            ));
        }

        let split = DatasetSplit {
            source_path: dataset_path.to_string_lossy().to_string(),
            train_path: train_path.to_string_lossy().to_string(),
            val_path: val_path.to_string_lossy().to_string(),
            infer_path: infer_path.to_string_lossy().to_string(),
            total_rows,
            max_rows,
            train_rows,
            val_rows,
            infer_rows,
            iteration: plan.iteration,
        };

        let content = serde_json::to_string(&split).unwrap_or_default();
        AgentEffect::with_proposal(proposal(
            self.name(),
            ContextKey::Signals,
            format!("dataset-split-{}", plan.iteration),
            content,
        ))
    }
}

#[derive(Debug)]
pub struct ModelTrainingAgent {
    model_dir: PathBuf,
}

impl ModelTrainingAgent {
    pub fn new(model_dir: PathBuf) -> Self {
        Self { model_dir }
    }

    fn model_path(&self) -> PathBuf {
        self.model_dir.join("baseline_mean.json")
    }
}

#[async_trait::async_trait]
impl Suggestor for ModelTrainingAgent {
    fn name(&self) -> &str {
        "ModelTrainingAgent (Baseline)"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        if !ctx.has(ContextKey::Signals) {
            return false;
        }
        let split = match read_latest_split_from_ctx(ctx) {
            Ok(split) => split,
            Err(_) => return false,
        };
        !has_model_for_iteration(ctx, split.iteration)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let split = match read_latest_split_from_ctx(ctx) {
            Ok(split) => split,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "model-training-error",
                    err.to_string(),
                ));
            }
        };

        if let Err(err) = create_dir_all(&self.model_dir) {
            return AgentEffect::with_proposal(proposal(
                self.name(),
                ContextKey::Diagnostic,
                "model-training-error",
                err.to_string(),
            ));
        }

        let raw_train_df = match load_dataframe(Path::new(&split.train_path)) {
            Ok(df) => df,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "model-training-error",
                    err.to_string(),
                ));
            }
        };

        // Apply FeatureSpec transformation if available
        let train_df = match read_feature_spec_from_ctx(ctx, split.iteration) {
            Some(spec) => match apply_feature_spec(&raw_train_df, &spec) {
                Ok(df) => df,
                Err(err) => {
                    return AgentEffect::with_proposal(proposal(
                        self.name(),
                        ContextKey::Diagnostic,
                        "model-training-error",
                        format!("feature spec application failed: {}", err),
                    ));
                }
            },
            None => raw_train_df,
        };

        let (target_name, target) = match select_target_column(&train_df) {
            Ok(value) => value,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "model-training-error",
                    err.to_string(),
                ));
            }
        };

        let mean = match mean_of_series(&target) {
            Ok(value) => value,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "model-training-error",
                    err.to_string(),
                ));
            }
        };

        let model = BaselineModel {
            target_column: target_name.clone(),
            mean,
        };

        let model_path = self.model_path();
        if let Err(err) = write_json(&model_path, &model) {
            return AgentEffect::with_proposal(proposal(
                self.name(),
                ContextKey::Diagnostic,
                "model-training-error",
                err.to_string(),
            ));
        }

        let meta = ModelMetadata {
            model_path: model_path.to_string_lossy().to_string(),
            target_column: target_name,
            train_rows: split.train_rows,
            baseline_mean: mean,
            iteration: split.iteration,
        };

        let content = serde_json::to_string(&meta).unwrap_or_default();
        AgentEffect::with_proposal(proposal(
            self.name(),
            ContextKey::Strategies,
            format!("trained-model-{}", split.iteration),
            content,
        ))
    }
}

#[derive(Debug, Default)]
pub struct ModelEvaluationAgent;

impl ModelEvaluationAgent {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Default)]
pub struct ModelRegistryAgent;

impl ModelRegistryAgent {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Suggestor for ModelRegistryAgent {
    fn name(&self) -> &str {
        "ModelRegistryAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies, ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Strategies)
            && ctx.has(ContextKey::Evaluations)
            && match read_latest_model_meta_from_ctx(ctx) {
                Ok(meta) => !has_registry_record_for_iteration(ctx, meta.iteration),
                Err(_) => false,
            }
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let meta = match read_latest_model_meta_from_ctx(ctx) {
            Ok(meta) => meta,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "model-registry-error",
                    err.to_string(),
                ));
            }
        };

        let report = latest_evaluation_report(ctx, meta.iteration);
        let mut metrics = HashMap::new();
        if let Some(report) = report {
            metrics.insert(report.metric, report.value);
            metrics.insert("success_ratio".to_string(), report.success_ratio);
        }

        let record = ModelRegistryRecord {
            kind: "model_registry".to_string(),
            iteration: meta.iteration,
            model_path: meta.model_path,
            metrics,
            provenance: "training_flow".to_string(),
        };

        let content = serde_json::to_string(&record).unwrap_or_default();
        AgentEffect::with_proposal(proposal(
            self.name(),
            ContextKey::Strategies,
            format!("model-registry-{}", record.iteration),
            content,
        ))
    }
}

#[derive(Debug, Default)]
pub struct MonitoringAgent;

impl MonitoringAgent {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Suggestor for MonitoringAgent {
    fn name(&self) -> &str {
        "MonitoringAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Evaluations)
            && match latest_evaluation_report(ctx, 0) {
                Some(report) => !has_monitoring_report_for_iteration(ctx, report.iteration),
                None => false,
            }
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let report = match latest_evaluation_report(ctx, 0) {
            Some(report) => report,
            None => return AgentEffect::empty(),
        };

        let status = if report.success_ratio >= 0.75 {
            "healthy"
        } else {
            "needs_attention"
        };

        let monitoring = MonitoringReport {
            kind: "monitoring".to_string(),
            iteration: report.iteration,
            metric: report.metric,
            value: report.value,
            baseline: report.mean_abs_target,
            status: status.to_string(),
        };

        let content = serde_json::to_string(&monitoring).unwrap_or_default();
        AgentEffect::with_proposal(proposal(
            self.name(),
            ContextKey::Evaluations,
            format!("monitoring-{}", report.iteration),
            content,
        ))
    }
}

#[derive(Debug, Default)]
pub struct DeploymentAgent;

impl DeploymentAgent {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Suggestor for DeploymentAgent {
    fn name(&self) -> &str {
        "DeploymentAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations, ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Evaluations)
            && ctx.has(ContextKey::Strategies)
            && match latest_evaluation_report(ctx, 0) {
                Some(report) => !has_deployment_decision_for_iteration(ctx, report.iteration),
                None => false,
            }
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let report = match latest_evaluation_report(ctx, 0) {
            Some(report) => report,
            None => return AgentEffect::empty(),
        };

        let quality_threshold = read_latest_plan_from_ctx(ctx)
            .map(|plan| plan.quality_threshold)
            .unwrap_or(0.75);

        let (action, retrain, reason) = if report.success_ratio >= quality_threshold {
            ("deploy", false, "meets quality threshold")
        } else {
            ("hold", true, "below quality threshold")
        };

        let decision = DeploymentDecision {
            kind: "deployment_decision".to_string(),
            iteration: report.iteration,
            action: action.to_string(),
            reason: reason.to_string(),
            retrain,
        };

        let content = serde_json::to_string(&decision).unwrap_or_default();
        AgentEffect::with_proposal(proposal(
            self.name(),
            ContextKey::Strategies,
            format!("deployment-{}", report.iteration),
            content,
        ))
    }
}

#[async_trait::async_trait]
impl Suggestor for ModelEvaluationAgent {
    fn name(&self) -> &str {
        "ModelEvaluationAgent (MAE)"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals, ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Signals)
            && ctx.has(ContextKey::Strategies)
            && match read_latest_split_from_ctx(ctx) {
                Ok(split) => !has_evaluation_for_iteration(ctx, split.iteration),
                Err(_) => false,
            }
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let split = match read_latest_split_from_ctx(ctx) {
            Ok(split) => split,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "model-eval-error",
                    err.to_string(),
                ));
            }
        };

        let model = match read_model_from_ctx(ctx) {
            Ok(model) => model,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "model-eval-error",
                    err.to_string(),
                ));
            }
        };

        let raw_val_df = match load_dataframe(Path::new(&split.val_path)) {
            Ok(df) => df,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "model-eval-error",
                    err.to_string(),
                ));
            }
        };

        // Apply FeatureSpec transformation if available
        let val_df = match read_feature_spec_from_ctx(ctx, split.iteration) {
            Some(spec) => apply_feature_spec(&raw_val_df, &spec).unwrap_or(raw_val_df),
            None => raw_val_df,
        };

        let target = match get_numeric_series(&val_df, &model.target_column) {
            Ok(series) => series,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "model-eval-error",
                    err.to_string(),
                ));
            }
        };

        let mae = match mean_abs_error(&target, model.mean) {
            Ok(value) => value,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "model-eval-error",
                    err.to_string(),
                ));
            }
        };

        let mean_abs = match mean_abs_value(&target) {
            Ok(value) => value,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "model-eval-error",
                    err.to_string(),
                ));
            }
        };

        let success_ratio = if mean_abs > 0.0 {
            (1.0 - (mae / mean_abs)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let report = EvaluationReport {
            model_path: read_model_path_from_ctx(ctx).unwrap_or_default(),
            metric: "mae".to_string(),
            value: mae,
            mean_abs_target: mean_abs,
            success_ratio,
            val_rows: split.val_rows,
            iteration: split.iteration,
        };

        let content = serde_json::to_string(&report).unwrap_or_default();
        AgentEffect::with_proposal(proposal(
            self.name(),
            ContextKey::Evaluations,
            format!("model-eval-{}", split.iteration),
            content,
        ))
    }
}

#[derive(Debug)]
pub struct SampleInferenceAgent {
    pub max_rows: usize,
}

impl SampleInferenceAgent {
    pub fn new(max_rows: usize) -> Self {
        Self { max_rows }
    }
}

#[async_trait::async_trait]
impl Suggestor for SampleInferenceAgent {
    fn name(&self) -> &str {
        "SampleInferenceAgent (Baseline)"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals, ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Signals)
            && ctx.has(ContextKey::Strategies)
            && match read_latest_split_from_ctx(ctx) {
                Ok(split) => !has_inference_for_iteration(ctx, split.iteration),
                Err(_) => false,
            }
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let split = match read_latest_split_from_ctx(ctx) {
            Ok(split) => split,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "model-infer-error",
                    err.to_string(),
                ));
            }
        };

        let model = match read_model_from_ctx(ctx) {
            Ok(model) => model,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "model-infer-error",
                    err.to_string(),
                ));
            }
        };

        let infer_df = match load_dataframe(Path::new(&split.infer_path)) {
            Ok(df) => df,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "model-infer-error",
                    err.to_string(),
                ));
            }
        };

        let target = match get_numeric_series(&infer_df, &model.target_column) {
            Ok(series) => series,
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "model-infer-error",
                    err.to_string(),
                ));
            }
        };

        let sample_rows = self.max_rows.min(infer_df.height().max(1));
        let actuals = match target.f64() {
            Ok(series) => series
                .into_no_null_iter()
                .take(sample_rows)
                .collect::<Vec<_>>(),
            Err(err) => {
                return AgentEffect::with_proposal(proposal(
                    self.name(),
                    ContextKey::Diagnostic,
                    "model-infer-error",
                    err.to_string(),
                ));
            }
        };

        let predictions = vec![model.mean; actuals.len()];
        let sample = InferenceSample {
            model_path: read_model_path_from_ctx(ctx).unwrap_or_default(),
            target_column: model.target_column,
            rows: actuals.len(),
            predictions,
            actuals,
            iteration: split.iteration,
        };

        let content = serde_json::to_string(&sample).unwrap_or_default();
        AgentEffect::with_proposal(proposal(
            self.name(),
            ContextKey::Hypotheses,
            format!("inference-sample-{}", split.iteration),
            content,
        ))
    }
}

fn download_dataset_if_missing(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    let response = reqwest::blocking::get(DATASET_URL)?;
    let content = response.bytes()?;

    let mut file = File::create(path)?;
    file.write_all(&content)?;

    Ok(())
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

fn write_parquet(df: &DataFrame, path: &Path) -> Result<()> {
    let mut file = File::create(path)?;
    let mut owned = df.clone();
    ParquetWriter::new(&mut file).finish(&mut owned)?;
    Ok(())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let content = serde_json::to_string_pretty(value)?;
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

fn read_latest_split_from_ctx(ctx: &dyn Context) -> Result<DatasetSplit> {
    let facts = ctx.get(ContextKey::Signals);
    let mut latest: Option<DatasetSplit> = None;
    for fact in facts {
        if let Ok(split) = serde_json::from_str::<DatasetSplit>(&fact.content) {
            let should_replace = match &latest {
                Some(current) => split.iteration > current.iteration,
                None => true,
            };
            if should_replace {
                latest = Some(split);
            }
        }
    }
    latest.ok_or_else(|| anyhow!("missing dataset split"))
}

fn read_model_path_from_ctx(ctx: &dyn Context) -> Result<String> {
    let meta = read_latest_model_meta_from_ctx(ctx)?;
    Ok(meta.model_path)
}

fn read_model_from_ctx(ctx: &dyn Context) -> Result<BaselineModel> {
    let model_path = read_model_path_from_ctx(ctx)?;
    let content = std::fs::read_to_string(model_path)?;
    let model = serde_json::from_str(&content)?;
    Ok(model)
}

fn read_latest_model_meta_from_ctx(ctx: &dyn Context) -> Result<ModelMetadata> {
    let facts = ctx.get(ContextKey::Strategies);
    let mut latest: Option<ModelMetadata> = None;
    for fact in facts {
        if let Ok(meta) = serde_json::from_str::<ModelMetadata>(&fact.content) {
            let should_replace = match &latest {
                Some(current) => meta.iteration > current.iteration,
                None => true,
            };
            if should_replace {
                latest = Some(meta);
            }
        }
    }
    latest.ok_or_else(|| anyhow!("missing model metadata"))
}

fn read_latest_plan_from_ctx(ctx: &dyn Context) -> Option<TrainingPlan> {
    let facts = ctx.get(ContextKey::Constraints);
    let mut latest: Option<TrainingPlan> = None;
    for fact in facts {
        if let Ok(plan) = serde_json::from_str::<TrainingPlan>(&fact.content) {
            let should_replace = match &latest {
                Some(current) => plan.iteration > current.iteration,
                None => true,
            };
            if should_replace {
                latest = Some(plan);
            }
        }
    }
    latest
}

fn has_split_for_iteration(ctx: &dyn Context, iteration: usize) -> bool {
    ctx.get(ContextKey::Signals).iter().any(|fact| {
        serde_json::from_str::<DatasetSplit>(&fact.content)
            .map(|split| split.iteration == iteration)
            .unwrap_or(false)
    })
}

fn has_model_for_iteration(ctx: &dyn Context, iteration: usize) -> bool {
    ctx.get(ContextKey::Strategies).iter().any(|fact| {
        serde_json::from_str::<ModelMetadata>(&fact.content)
            .map(|meta| meta.iteration == iteration)
            .unwrap_or(false)
    })
}

fn has_evaluation_for_iteration(ctx: &dyn Context, iteration: usize) -> bool {
    ctx.get(ContextKey::Evaluations).iter().any(|fact| {
        serde_json::from_str::<EvaluationReport>(&fact.content)
            .map(|report| report.iteration == iteration)
            .unwrap_or(false)
    })
}

fn has_inference_for_iteration(ctx: &dyn Context, iteration: usize) -> bool {
    ctx.get(ContextKey::Hypotheses).iter().any(|fact| {
        serde_json::from_str::<InferenceSample>(&fact.content)
            .map(|sample| sample.iteration == iteration)
            .unwrap_or(false)
    })
}

fn has_data_quality_for_iteration(ctx: &dyn Context, iteration: usize) -> bool {
    ctx.get(ContextKey::Signals).iter().any(|fact| {
        serde_json::from_str::<DataQualityReport>(&fact.content)
            .map(|report| report.iteration == iteration)
            .unwrap_or(false)
    })
}

fn has_feature_spec_for_iteration(ctx: &dyn Context, iteration: usize) -> bool {
    ctx.get(ContextKey::Constraints).iter().any(|fact| {
        serde_json::from_str::<FeatureSpec>(&fact.content)
            .map(|spec| spec.iteration == iteration)
            .unwrap_or(false)
    })
}

fn has_hyperparam_result_for_iteration(ctx: &dyn Context, iteration: usize) -> bool {
    ctx.get(ContextKey::Evaluations).iter().any(|fact| {
        serde_json::from_str::<HyperparameterSearchResult>(&fact.content)
            .map(|result| result.iteration == iteration)
            .unwrap_or(false)
    })
}

fn has_registry_record_for_iteration(ctx: &dyn Context, iteration: usize) -> bool {
    ctx.get(ContextKey::Strategies).iter().any(|fact| {
        serde_json::from_str::<ModelRegistryRecord>(&fact.content)
            .map(|record| record.iteration == iteration)
            .unwrap_or(false)
    })
}

fn has_monitoring_report_for_iteration(ctx: &dyn Context, iteration: usize) -> bool {
    ctx.get(ContextKey::Evaluations).iter().any(|fact| {
        serde_json::from_str::<MonitoringReport>(&fact.content)
            .map(|report| report.iteration == iteration)
            .unwrap_or(false)
    })
}

fn has_deployment_decision_for_iteration(ctx: &dyn Context, iteration: usize) -> bool {
    ctx.get(ContextKey::Strategies).iter().any(|fact| {
        serde_json::from_str::<DeploymentDecision>(&fact.content)
            .map(|decision| decision.iteration == iteration)
            .unwrap_or(false)
    })
}

fn latest_evaluation_report(ctx: &dyn Context, iteration: usize) -> Option<EvaluationReport> {
    let mut latest: Option<EvaluationReport> = None;
    for fact in ctx.get(ContextKey::Evaluations) {
        if let Ok(report) = serde_json::from_str::<EvaluationReport>(&fact.content) {
            if iteration > 0 {
                if report.iteration == iteration {
                    return Some(report);
                }
            } else if latest
                .as_ref()
                .map(|current| report.iteration > current.iteration)
                .unwrap_or(true)
            {
                latest = Some(report);
            }
        }
    }
    if iteration > 0 { None } else { latest }
}

fn latest_data_quality_before_iteration(
    ctx: &dyn Context,
    iteration: usize,
) -> Option<DataQualityReport> {
    let mut latest: Option<DataQualityReport> = None;
    for fact in ctx.get(ContextKey::Signals) {
        if let Ok(report) = serde_json::from_str::<DataQualityReport>(&fact.content) {
            if report.iteration < iteration
                && latest
                    .as_ref()
                    .map(|current| report.iteration > current.iteration)
                    .unwrap_or(true)
            {
                latest = Some(report);
            }
        }
    }
    latest
}

fn drift_score_from_ctx(
    ctx: &dyn Context,
    iteration: usize,
    numeric_means: &HashMap<String, f64>,
) -> Option<f64> {
    let previous = latest_data_quality_before_iteration(ctx, iteration)?;
    let mut total_delta = 0.0;
    let mut count = 0usize;
    for (name, mean) in numeric_means {
        if let Some(prev_mean) = previous.numeric_means.get(name) {
            total_delta += (mean - prev_mean).abs();
            count += 1;
        }
    }
    if count == 0 {
        None
    } else {
        Some(total_delta / count as f64)
    }
}

fn compute_numeric_stats(series: &Series) -> Result<(f64, f64, usize)> {
    let casted = series.cast(&DataType::Float64)?;
    let values: Vec<f64> = casted
        .f64()
        .context("numeric series not f64")?
        .into_no_null_iter()
        .collect();
    if values.is_empty() {
        return Err(anyhow!("no numeric values to compute stats"));
    }

    let mut total = 0.0;
    for value in &values {
        total += *value;
    }
    let mean = total / values.len() as f64;

    let mut variance_sum = 0.0;
    for value in &values {
        let diff = *value - mean;
        variance_sum += diff * diff;
    }
    let std = (variance_sum / values.len() as f64).sqrt();

    let outliers = if std > 0.0 {
        values
            .iter()
            .filter(|value| (*value - mean).abs() > 3.0 * std)
            .count()
    } else {
        0
    };

    Ok((mean, std, outliers))
}

fn split_feature_columns(df: &DataFrame, target: &str) -> (Vec<String>, Vec<String>) {
    let mut numeric = Vec::new();
    let mut categorical = Vec::new();
    for series in df.get_columns() {
        let name = series.name();
        if name == target {
            continue;
        }
        if is_numeric_dtype(series.dtype()) {
            numeric.push(name.to_string());
        } else {
            categorical.push(name.to_string());
        }
    }
    (numeric, categorical)
}

fn select_target_column(df: &DataFrame) -> Result<(String, Series)> {
    if let Ok(col) = df.column(TARGET_COLUMN) {
        return Ok((
            TARGET_COLUMN.to_string(),
            col.as_materialized_series().clone(),
        ));
    }

    let mut numeric = df
        .get_columns()
        .iter()
        .filter(|series| is_numeric_dtype(series.dtype()))
        .cloned()
        .collect::<Vec<_>>();

    let fallback = numeric
        .pop()
        .ok_or_else(|| anyhow!("no numeric columns available for target"))?;
    let series = fallback.as_materialized_series().clone();
    Ok((series.name().to_string(), series))
}

fn get_numeric_series(df: &DataFrame, name: &str) -> Result<Series> {
    let series = df
        .column(name)
        .map_err(|_| anyhow!("missing target column {}", name))?
        .as_materialized_series();
    let casted = series.cast(&DataType::Float64)?;
    Ok(casted)
}

fn mean_of_series(series: &Series) -> Result<f64> {
    let casted = series.cast(&DataType::Float64)?;
    let values = casted
        .f64()
        .context("target column not f64")?
        .into_no_null_iter();
    let mut total = 0.0;
    let mut count = 0usize;
    for value in values {
        total += value;
        count += 1;
    }
    if count == 0 {
        return Err(anyhow!("no values to compute mean"));
    }
    Ok(total / count as f64)
}

fn mean_abs_error(target: &Series, mean: f64) -> Result<f64> {
    let casted = target.cast(&DataType::Float64)?;
    let values = casted
        .f64()
        .context("target column not f64")?
        .into_no_null_iter();
    let mut total = 0.0;
    let mut count = 0usize;
    for value in values {
        total += (value - mean).abs();
        count += 1;
    }
    if count == 0 {
        return Err(anyhow!("no values to evaluate"));
    }
    Ok(total / count as f64)
}

fn mean_abs_value(target: &Series) -> Result<f64> {
    let casted = target.cast(&DataType::Float64)?;
    let values = casted
        .f64()
        .context("target column not f64")?
        .into_no_null_iter();
    let mut total = 0.0;
    let mut count = 0usize;
    for value in values {
        total += value.abs();
        count += 1;
    }
    if count == 0 {
        return Err(anyhow!("no values to evaluate"));
    }
    Ok(total / count as f64)
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

/// Read the latest FeatureSpec from context for a given iteration
fn read_feature_spec_from_ctx(ctx: &dyn Context, iteration: usize) -> Option<FeatureSpec> {
    ctx.get(ContextKey::Constraints).iter().find_map(|fact| {
        serde_json::from_str::<FeatureSpec>(&fact.content)
            .ok()
            .filter(|spec| spec.iteration == iteration)
    })
}

/// Apply a FeatureSpec to a DataFrame, creating interaction features and normalizing
pub fn apply_feature_spec(df: &DataFrame, spec: &FeatureSpec) -> Result<DataFrame> {
    let mut result = df.clone();

    // Apply feature interactions
    for interaction in &spec.interactions {
        let left_col = result
            .column(&interaction.left)
            .map_err(|_| anyhow!("missing column {} for interaction", interaction.left))?
            .cast(&DataType::Float64)?;
        let right_col = result
            .column(&interaction.right)
            .map_err(|_| anyhow!("missing column {} for interaction", interaction.right))?
            .cast(&DataType::Float64)?;

        let left_vals = left_col.f64().context("left column not f64")?;
        let right_vals = right_col.f64().context("right column not f64")?;

        let interaction_series = match interaction.op.as_str() {
            "multiply" => left_vals * right_vals,
            "add" => left_vals + right_vals,
            "subtract" => left_vals - right_vals,
            "divide" => {
                // Safe division: use map to handle division safely
                left_vals
                    .into_iter()
                    .zip(right_vals.into_iter())
                    .map(|(l, r)| match (l, r) {
                        (Some(lv), Some(rv)) if rv.abs() > 1e-10 => Some(lv / rv),
                        _ => None,
                    })
                    .collect::<Float64Chunked>()
            }
            _ => return Err(anyhow!("unsupported interaction op: {}", interaction.op)),
        };

        let named_series = interaction_series.with_name(interaction.name.clone().into());
        result = result
            .hstack(&[named_series.into_series().into()])
            .context("failed to add interaction column")?;
    }

    // Apply normalization to numeric features
    if spec.normalization == "standardize" {
        for col_name in &spec.numeric_features {
            if let Ok(col) = result.column(col_name) {
                let casted = col.cast(&DataType::Float64)?;
                let values = casted.f64().context("column not f64")?;

                // Compute mean and std
                let (mean, std) = compute_mean_std(values)?;

                if std > 0.0 {
                    // Standardize: (x - mean) / std
                    let standardized = (values - mean) / std;
                    let named = standardized.with_name(col_name.clone().into());

                    // Replace the column
                    result = result.drop(col_name)?;
                    result = result
                        .hstack(&[named.into_series().into()])
                        .context("failed to replace standardized column")?;
                }
            }
        }
    }

    Ok(result)
}

fn compute_mean_std(values: &ChunkedArray<Float64Type>) -> Result<(f64, f64)> {
    let vals: Vec<f64> = values.into_no_null_iter().collect();
    if vals.is_empty() {
        return Err(anyhow!("no values for mean/std computation"));
    }

    let mean = vals.iter().sum::<f64>() / vals.len() as f64;
    let variance = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / vals.len() as f64;
    let std = variance.sqrt();

    Ok((mean, std))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn proposal_helper_builds_correct_fact() {
        let p = proposal("my-agent", ContextKey::Diagnostic, "id-1", "content-1");
        assert_eq!(p.provenance, "my-agent");
        assert_eq!(p.key, ContextKey::Diagnostic);
        assert_eq!(p.id, "id-1");
        assert_eq!(p.content, "content-1");
    }

    #[test]
    fn training_plan_serde_roundtrip() {
        let plan = TrainingPlan {
            iteration: 3,
            max_rows: 1000,
            train_fraction: 0.7,
            val_fraction: 0.2,
            infer_fraction: 0.1,
            quality_threshold: 0.8,
        };
        let json = serde_json::to_string(&plan).unwrap();
        let restored: TrainingPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.iteration, 3);
        assert_eq!(restored.max_rows, 1000);
        assert!((restored.train_fraction - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn dataset_split_serde_roundtrip() {
        let split = DatasetSplit {
            source_path: "/data/src.parquet".into(),
            train_path: "/data/train.parquet".into(),
            val_path: "/data/val.parquet".into(),
            infer_path: "/data/infer.parquet".into(),
            total_rows: 1000,
            max_rows: 800,
            train_rows: 640,
            val_rows: 120,
            infer_rows: 40,
            iteration: 1,
        };
        let json = serde_json::to_string(&split).unwrap();
        let restored: DatasetSplit = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.total_rows, 1000);
        assert_eq!(
            restored.train_rows + restored.val_rows + restored.infer_rows,
            800
        );
    }

    #[test]
    fn baseline_model_serde_roundtrip() {
        let model = BaselineModel {
            target_column: "price".into(),
            mean: 42.5,
        };
        let json = serde_json::to_string(&model).unwrap();
        let restored: BaselineModel = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.target_column, "price");
        assert!((restored.mean - 42.5).abs() < f64::EPSILON);
    }

    #[test]
    fn evaluation_report_success_ratio_bounds() {
        let report = EvaluationReport {
            model_path: "/model".into(),
            metric: "mae".into(),
            value: 10.0,
            mean_abs_target: 100.0,
            success_ratio: 0.9,
            val_rows: 50,
            iteration: 1,
        };
        assert!(report.success_ratio >= 0.0 && report.success_ratio <= 1.0);
    }

    #[test]
    fn feature_interaction_construction() {
        let fi = FeatureInteraction {
            name: "a_x_b".into(),
            left: "a".into(),
            right: "b".into(),
            op: "multiply".into(),
        };
        assert_eq!(fi.name, "a_x_b");
        assert_eq!(fi.op, "multiply");
    }

    #[test]
    fn feature_spec_serde_roundtrip() {
        let spec = FeatureSpec {
            kind: "feature_spec".into(),
            iteration: 2,
            target_column: "target".into(),
            numeric_features: vec!["a".into(), "b".into()],
            categorical_features: vec!["c".into()],
            normalization: "standardize".into(),
            interactions: vec![],
        };
        let json = serde_json::to_string(&spec).unwrap();
        let restored: FeatureSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.numeric_features.len(), 2);
        assert_eq!(restored.categorical_features.len(), 1);
    }

    #[test]
    fn hyperparam_search_plan_construction() {
        let mut params = HashMap::new();
        params.insert("lr".to_string(), vec![0.001, 0.01]);
        let plan = HyperparameterSearchPlan {
            kind: "hyperparam_plan".into(),
            iteration: 1,
            max_trials: 10,
            early_stopping: true,
            params,
        };
        assert_eq!(plan.max_trials, 10);
        assert!(plan.early_stopping);
        assert_eq!(plan.params["lr"].len(), 2);
    }

    #[test]
    fn hyperparam_search_result_serde_roundtrip() {
        let mut best = HashMap::new();
        best.insert("lr".to_string(), 0.01);
        let result = HyperparameterSearchResult {
            kind: "hyperparam_result".into(),
            iteration: 1,
            best_params: best,
            score: 0.85,
        };
        let json = serde_json::to_string(&result).unwrap();
        let restored: HyperparameterSearchResult = serde_json::from_str(&json).unwrap();
        assert!((restored.score - 0.85).abs() < f64::EPSILON);
        assert!((restored.best_params["lr"] - 0.01).abs() < f64::EPSILON);
    }

    #[test]
    fn model_registry_record_construction() {
        let mut metrics = HashMap::new();
        metrics.insert("mae".to_string(), 5.0);
        let record = ModelRegistryRecord {
            kind: "model_registry".into(),
            iteration: 1,
            model_path: "/models/v1.json".into(),
            metrics,
            provenance: "test".into(),
        };
        assert_eq!(record.metrics["mae"], 5.0);
    }

    #[test]
    fn monitoring_report_status_values() {
        let healthy = MonitoringReport {
            kind: "monitoring".into(),
            iteration: 1,
            metric: "mae".into(),
            value: 5.0,
            baseline: 100.0,
            status: "healthy".into(),
        };
        assert_eq!(healthy.status, "healthy");

        let needs_attention = MonitoringReport {
            status: "needs_attention".into(),
            ..healthy.clone()
        };
        assert_eq!(needs_attention.status, "needs_attention");
    }

    #[test]
    fn deployment_decision_deploy_vs_hold() {
        let deploy = DeploymentDecision {
            kind: "deployment_decision".into(),
            iteration: 1,
            action: "deploy".into(),
            reason: "meets threshold".into(),
            retrain: false,
        };
        assert!(!deploy.retrain);

        let hold = DeploymentDecision {
            action: "hold".into(),
            retrain: true,
            ..deploy.clone()
        };
        assert!(hold.retrain);
        assert_eq!(hold.action, "hold");
    }

    #[test]
    fn data_validation_agent_default() {
        let agent = DataValidationAgent::default();
        let agent2 = DataValidationAgent::new();
        assert_eq!(format!("{:?}", agent), format!("{:?}", agent2));
    }

    #[test]
    fn feature_engineering_agent_default() {
        let agent = FeatureEngineeringAgent::default();
        let agent2 = FeatureEngineeringAgent::new();
        assert_eq!(format!("{:?}", agent), format!("{:?}", agent2));
    }

    #[test]
    fn model_evaluation_agent_default() {
        let agent = ModelEvaluationAgent::default();
        let agent2 = ModelEvaluationAgent::new();
        assert_eq!(format!("{:?}", agent), format!("{:?}", agent2));
    }

    #[test]
    fn model_registry_agent_default() {
        let agent = ModelRegistryAgent::default();
        let agent2 = ModelRegistryAgent::new();
        assert_eq!(format!("{:?}", agent), format!("{:?}", agent2));
    }

    #[test]
    fn monitoring_agent_default() {
        let agent = MonitoringAgent::default();
        let agent2 = MonitoringAgent::new();
        assert_eq!(format!("{:?}", agent), format!("{:?}", agent2));
    }

    #[test]
    fn deployment_agent_default() {
        let agent = DeploymentAgent::default();
        let agent2 = DeploymentAgent::new();
        assert_eq!(format!("{:?}", agent), format!("{:?}", agent2));
    }

    #[test]
    fn dataset_agent_paths() {
        let agent = DatasetAgent::new(PathBuf::from("/tmp/data"));
        assert_eq!(
            agent.dataset_path(),
            PathBuf::from("/tmp/data/california_housing_train.parquet")
        );
        let (train, val, infer) = agent.split_paths();
        assert_eq!(train, PathBuf::from("/tmp/data/train.parquet"));
        assert_eq!(val, PathBuf::from("/tmp/data/val.parquet"));
        assert_eq!(infer, PathBuf::from("/tmp/data/infer.parquet"));
    }

    #[test]
    fn model_training_agent_model_path() {
        let agent = ModelTrainingAgent::new(PathBuf::from("/tmp/models"));
        assert_eq!(
            agent.model_path(),
            PathBuf::from("/tmp/models/baseline_mean.json")
        );
    }

    #[test]
    fn sample_inference_agent_construction() {
        let agent = SampleInferenceAgent::new(100);
        assert_eq!(agent.max_rows, 100);
    }

    #[test]
    fn hyperparameter_search_agent_construction() {
        let agent = HyperparameterSearchAgent::new(50);
        assert_eq!(agent.max_trials, 50);
    }

    #[test]
    fn is_numeric_dtype_comprehensive() {
        assert!(is_numeric_dtype(&DataType::Float32));
        assert!(is_numeric_dtype(&DataType::Float64));
        assert!(is_numeric_dtype(&DataType::Int64));
        assert!(!is_numeric_dtype(&DataType::String));
        assert!(!is_numeric_dtype(&DataType::Boolean));
    }

    #[test]
    fn split_feature_columns_separates_types() {
        let df = df! {
            "num1" => [1.0, 2.0],
            "num2" => [3i32, 4],
            "cat1" => ["a", "b"],
            "target" => [10.0, 20.0]
        }
        .unwrap();
        let (numeric, categorical) = split_feature_columns(&df, "target");
        assert!(numeric.contains(&"num1".to_string()));
        assert!(numeric.contains(&"num2".to_string()));
        assert!(categorical.contains(&"cat1".to_string()));
        assert!(!numeric.contains(&"target".to_string()));
        assert!(!categorical.contains(&"target".to_string()));
    }

    #[test]
    fn select_target_column_prefers_named_target() {
        let df = df! {
            "x" => [1.0, 2.0],
            "median_house_value" => [100.0, 200.0]
        }
        .unwrap();
        let (name, _) = select_target_column(&df).unwrap();
        assert_eq!(name, "median_house_value");
    }

    #[test]
    fn select_target_column_falls_back_to_last_numeric() {
        let df = df! {
            "a" => [1.0, 2.0],
            "b" => [3.0, 4.0]
        }
        .unwrap();
        let (name, _) = select_target_column(&df).unwrap();
        assert_eq!(name, "b");
    }

    #[test]
    fn select_target_column_fails_with_no_numeric() {
        let df = df! {
            "text" => ["a", "b"]
        }
        .unwrap();
        assert!(select_target_column(&df).is_err());
    }

    #[test]
    fn mean_of_series_computes_correctly() {
        let series = Series::new("v".into(), &[2.0f64, 4.0, 6.0]);
        let mean = mean_of_series(&series).unwrap();
        assert!((mean - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn mean_abs_error_computes_correctly() {
        let series = Series::new("v".into(), &[10.0f64, 20.0, 30.0]);
        let mae = mean_abs_error(&series, 20.0).unwrap();
        // |10-20| + |20-20| + |30-20| = 10+0+10 = 20, /3 = 6.666...
        assert!((mae - 20.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn mean_abs_value_computes_correctly() {
        let series = Series::new("v".into(), &[-5.0f64, 5.0, 10.0]);
        let mav = mean_abs_value(&series).unwrap();
        assert!((mav - 20.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn compute_numeric_stats_basic() {
        let series = Series::new("v".into(), &[2.0f64, 4.0, 6.0]);
        let casted = series.cast(&DataType::Float64).unwrap();
        let (mean, std, outliers) = compute_numeric_stats(&casted).unwrap();
        assert!((mean - 4.0).abs() < 1e-10);
        assert!(std > 0.0);
        assert_eq!(outliers, 0);
    }

    #[test]
    fn compute_numeric_stats_empty_fails() {
        let series = Series::new("v".into(), Vec::<f64>::new());
        assert!(compute_numeric_stats(&series).is_err());
    }

    #[test]
    fn compute_numeric_stats_constant_series() {
        let series = Series::new("v".into(), &[5.0f64, 5.0, 5.0]);
        let (mean, std, outliers) = compute_numeric_stats(&series).unwrap();
        assert!((mean - 5.0).abs() < f64::EPSILON);
        assert!((std - 0.0).abs() < f64::EPSILON);
        assert_eq!(outliers, 0);
    }

    #[test]
    fn data_quality_report_serde_roundtrip() {
        let mut missingness = HashMap::new();
        missingness.insert("a".to_string(), 0.1);
        let report = DataQualityReport {
            kind: "data_quality".into(),
            iteration: 1,
            source_path: "/data/train.parquet".into(),
            rows_checked: 100,
            missingness,
            numeric_means: HashMap::new(),
            outlier_counts: HashMap::new(),
            drift_score: Some(0.05),
        };
        let json = serde_json::to_string(&report).unwrap();
        let restored: DataQualityReport = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.rows_checked, 100);
        assert!((restored.drift_score.unwrap() - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn inference_sample_serde_roundtrip() {
        let sample = InferenceSample {
            model_path: "/model".into(),
            target_column: "target".into(),
            rows: 3,
            predictions: vec![1.0, 2.0, 3.0],
            actuals: vec![1.1, 2.1, 3.1],
            iteration: 1,
        };
        let json = serde_json::to_string(&sample).unwrap();
        let restored: InferenceSample = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.predictions.len(), 3);
        assert_eq!(restored.actuals.len(), 3);
    }

    #[test]
    fn apply_feature_spec_creates_interaction_column() {
        let df = df! {
            "a" => [1.0, 2.0, 3.0],
            "b" => [4.0, 5.0, 6.0],
            "target" => [10.0, 20.0, 30.0]
        }
        .unwrap();

        let spec = FeatureSpec {
            kind: "feature_spec".to_string(),
            iteration: 1,
            target_column: "target".to_string(),
            numeric_features: vec!["a".to_string(), "b".to_string()],
            categorical_features: vec![],
            normalization: "none".to_string(),
            interactions: vec![FeatureInteraction {
                name: "a_x_b".to_string(),
                left: "a".to_string(),
                right: "b".to_string(),
                op: "multiply".to_string(),
            }],
        };

        let result = apply_feature_spec(&df, &spec).unwrap();

        // Check interaction column exists
        assert!(result.column("a_x_b").is_ok());

        // Check values: 1*4=4, 2*5=10, 3*6=18
        let interaction = result.column("a_x_b").unwrap().f64().unwrap();
        let values: Vec<f64> = interaction.into_no_null_iter().collect();
        assert_eq!(values, vec![4.0, 10.0, 18.0]);
    }

    #[test]
    fn apply_feature_spec_standardizes_numeric_features() {
        let df = df! {
            "a" => [1.0, 2.0, 3.0, 4.0, 5.0],
            "target" => [10.0, 20.0, 30.0, 40.0, 50.0]
        }
        .unwrap();

        let spec = FeatureSpec {
            kind: "feature_spec".to_string(),
            iteration: 1,
            target_column: "target".to_string(),
            numeric_features: vec!["a".to_string()],
            categorical_features: vec![],
            normalization: "standardize".to_string(),
            interactions: vec![],
        };

        let result = apply_feature_spec(&df, &spec).unwrap();

        // Standardized values should have mean ~0 and std ~1
        let a_col = result.column("a").unwrap().f64().unwrap();
        let values: Vec<f64> = a_col.into_no_null_iter().collect();

        let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
        assert!(mean.abs() < 1e-10, "mean should be ~0, got {}", mean);

        let variance: f64 =
            values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        let std = variance.sqrt();
        assert!((std - 1.0).abs() < 1e-10, "std should be ~1, got {}", std);
    }

    #[test]
    fn apply_feature_spec_handles_add_operation() {
        let df = df! {
            "a" => [1.0, 2.0, 3.0],
            "b" => [10.0, 20.0, 30.0]
        }
        .unwrap();

        let spec = FeatureSpec {
            kind: "feature_spec".to_string(),
            iteration: 1,
            target_column: "target".to_string(),
            numeric_features: vec![],
            categorical_features: vec![],
            normalization: "none".to_string(),
            interactions: vec![FeatureInteraction {
                name: "a_plus_b".to_string(),
                left: "a".to_string(),
                right: "b".to_string(),
                op: "add".to_string(),
            }],
        };

        let result = apply_feature_spec(&df, &spec).unwrap();
        let col = result.column("a_plus_b").unwrap().f64().unwrap();
        let values: Vec<f64> = col.into_no_null_iter().collect();
        assert_eq!(values, vec![11.0, 22.0, 33.0]);
    }
}
