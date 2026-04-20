//! Negative tests: analytics packs reject invalid input and handle edge cases.

use converge_analytics::packs::{
    AnomalyDetectionPack, ClassificationPack, DescriptiveStatsPack, ForecastingPack, RankingPack,
    RegressionPack, SegmentationPack, SimilarityPack, TrendDetectionPack,
};
use converge_kernel::{Budget, ContextKey, ContextState, Engine};
use converge_optimization::packs::Pack;
use converge_optimization::suggestor::SolverSuggestor;

fn budget() -> Budget {
    Budget {
        max_cycles: 5,
        max_facts: 100,
    }
}

// ── Validation rejects invalid inputs at the Pack level ──

#[test]
fn anomaly_detection_rejects_empty_values() {
    let input = serde_json::json!({"values": [], "threshold": 2.0});
    assert!(AnomalyDetectionPack.validate_inputs(&input).is_err());
}

#[test]
fn anomaly_detection_rejects_negative_threshold() {
    let input = serde_json::json!({"values": [1.0, 2.0], "threshold": -1.0});
    assert!(AnomalyDetectionPack.validate_inputs(&input).is_err());
}

#[test]
fn anomaly_detection_rejects_mismatched_labels() {
    let input = serde_json::json!({
        "values": [1.0, 2.0, 3.0],
        "threshold": 2.0,
        "labels": ["a", "b"]
    });
    assert!(AnomalyDetectionPack.validate_inputs(&input).is_err());
}

#[test]
fn segmentation_rejects_empty_records() {
    let input = serde_json::json!({"records": [], "k": 2});
    assert!(SegmentationPack.validate_inputs(&input).is_err());
}

#[test]
fn segmentation_rejects_k_zero() {
    let input = serde_json::json!({"records": [[1.0, 2.0]], "k": 0});
    assert!(SegmentationPack.validate_inputs(&input).is_err());
}

#[test]
fn segmentation_rejects_k_exceeds_records() {
    let input = serde_json::json!({"records": [[1.0], [2.0]], "k": 5});
    assert!(SegmentationPack.validate_inputs(&input).is_err());
}

#[test]
fn segmentation_rejects_inconsistent_dimensions() {
    let input = serde_json::json!({"records": [[1.0, 2.0], [3.0]], "k": 2});
    assert!(SegmentationPack.validate_inputs(&input).is_err());
}

#[test]
fn ranking_rejects_empty_items() {
    let input = serde_json::json!({
        "items": [],
        "weights": [0.5],
        "higher_is_better": [true]
    });
    assert!(RankingPack.validate_inputs(&input).is_err());
}

#[test]
fn ranking_rejects_negative_weight() {
    let input = serde_json::json!({
        "items": [{"id": "a", "scores": [1.0]}],
        "weights": [-1.0],
        "higher_is_better": [true]
    });
    assert!(RankingPack.validate_inputs(&input).is_err());
}

#[test]
fn ranking_rejects_dimension_mismatch() {
    let input = serde_json::json!({
        "items": [{"id": "a", "scores": [1.0, 2.0]}],
        "weights": [0.5],
        "higher_is_better": [true]
    });
    assert!(RankingPack.validate_inputs(&input).is_err());
}

#[test]
fn forecasting_rejects_single_value() {
    let input = serde_json::json!({"values": [1.0], "horizon": 3, "alpha": 0.3});
    assert!(ForecastingPack.validate_inputs(&input).is_err());
}

#[test]
fn forecasting_rejects_zero_horizon() {
    let input = serde_json::json!({"values": [1.0, 2.0, 3.0], "horizon": 0, "alpha": 0.3});
    assert!(ForecastingPack.validate_inputs(&input).is_err());
}

#[test]
fn forecasting_rejects_alpha_out_of_range() {
    let input = serde_json::json!({"values": [1.0, 2.0, 3.0], "horizon": 1, "alpha": 1.5});
    assert!(ForecastingPack.validate_inputs(&input).is_err());
}

#[test]
fn classification_rejects_empty_records() {
    let input = serde_json::json!({
        "records": [],
        "weights": [1.0],
        "bias": 0.0,
        "threshold": 0.5
    });
    assert!(ClassificationPack.validate_inputs(&input).is_err());
}

#[test]
fn classification_rejects_threshold_out_of_range() {
    let input = serde_json::json!({
        "records": [[1.0]],
        "weights": [1.0],
        "bias": 0.0,
        "threshold": 2.0
    });
    assert!(ClassificationPack.validate_inputs(&input).is_err());
}

#[test]
fn regression_rejects_dimension_mismatch() {
    let input = serde_json::json!({
        "records": [[1.0, 2.0], [3.0]],
        "weights": [1.0, 2.0],
        "bias": 0.0
    });
    assert!(RegressionPack.validate_inputs(&input).is_err());
}

#[test]
fn similarity_rejects_single_item() {
    let input = serde_json::json!({
        "items": [{"id": "a", "features": [1.0]}],
        "metric": "cosine"
    });
    assert!(SimilarityPack.validate_inputs(&input).is_err());
}

#[test]
fn similarity_rejects_zero_top_k() {
    let input = serde_json::json!({
        "items": [
            {"id": "a", "features": [1.0]},
            {"id": "b", "features": [2.0]}
        ],
        "metric": "cosine",
        "top_k": 0
    });
    assert!(SimilarityPack.validate_inputs(&input).is_err());
}

#[test]
fn trend_detection_rejects_too_few_values() {
    let input = serde_json::json!({"values": [1.0, 2.0], "window": 3});
    assert!(TrendDetectionPack.validate_inputs(&input).is_err());
}

#[test]
fn trend_detection_rejects_window_exceeds_values() {
    let input = serde_json::json!({"values": [1.0, 2.0, 3.0], "window": 5});
    assert!(TrendDetectionPack.validate_inputs(&input).is_err());
}

#[test]
fn descriptive_stats_rejects_empty_values() {
    let input = serde_json::json!({"values": []});
    assert!(DescriptiveStatsPack.validate_inputs(&input).is_err());
}

#[test]
fn descriptive_stats_rejects_invalid_percentile() {
    let input = serde_json::json!({"values": [1.0, 2.0], "percentiles": [150.0]});
    assert!(DescriptiveStatsPack.validate_inputs(&input).is_err());
}

// ── Engine-level: invalid JSON seed → pack returns empty effect, engine converges ──

async fn run_with_garbage<P: Pack + 'static>(pack: P) {
    let mut engine = Engine::with_budget(budget());
    engine.register_suggestor(SolverSuggestor::new(
        pack,
        ContextKey::Seeds,
        ContextKey::Strategies,
    ));
    let mut ctx = ContextState::new();
    let _ = ctx.add_input(ContextKey::Seeds, "garbage", "not valid json {{{");
    let result = engine.run(ctx).await.expect("should converge gracefully");
    assert!(result.converged);
    // No strategies produced from invalid input
    assert!(result.context.get(ContextKey::Strategies).is_empty());
}

#[tokio::test]
async fn anomaly_detection_garbage_input_converges_empty() {
    run_with_garbage(AnomalyDetectionPack).await;
}

#[tokio::test]
async fn segmentation_garbage_input_converges_empty() {
    run_with_garbage(SegmentationPack).await;
}

#[tokio::test]
async fn ranking_garbage_input_converges_empty() {
    run_with_garbage(RankingPack).await;
}

#[tokio::test]
async fn forecasting_garbage_input_converges_empty() {
    run_with_garbage(ForecastingPack).await;
}

#[tokio::test]
async fn classification_garbage_input_converges_empty() {
    run_with_garbage(ClassificationPack).await;
}

#[tokio::test]
async fn regression_garbage_input_converges_empty() {
    run_with_garbage(RegressionPack).await;
}

#[tokio::test]
async fn similarity_garbage_input_converges_empty() {
    run_with_garbage(SimilarityPack).await;
}

#[tokio::test]
async fn trend_detection_garbage_input_converges_empty() {
    run_with_garbage(TrendDetectionPack).await;
}

#[tokio::test]
async fn descriptive_stats_garbage_input_converges_empty() {
    run_with_garbage(DescriptiveStatsPack).await;
}

// ── Idempotency: running same input twice yields same output (no duplication) ──

async fn run_idempotent<P: Pack + 'static>(pack: P, input: serde_json::Value) {
    let mut engine = Engine::with_budget(Budget {
        max_cycles: 10,
        max_facts: 100,
    });
    engine.register_suggestor(SolverSuggestor::new(
        pack,
        ContextKey::Seeds,
        ContextKey::Strategies,
    ));
    let mut ctx = ContextState::new();
    let _ = ctx.add_input(ContextKey::Seeds, "input-1", input.to_string());
    let result = engine.run(ctx).await.expect("should converge");

    // Should converge quickly (2 cycles: seed present → solve → output present → stop)
    assert!(result.converged);
    assert!(
        result.cycles <= 3,
        "took {} cycles, expected <= 3",
        result.cycles
    );
    // Exactly one strategy produced (no duplicates from re-running)
    assert_eq!(result.context.get(ContextKey::Strategies).len(), 1);
}

#[tokio::test]
async fn anomaly_detection_idempotent() {
    run_idempotent(
        AnomalyDetectionPack,
        serde_json::json!({"values": [1.0, 2.0, 3.0, 100.0], "threshold": 2.0}),
    )
    .await;
}

#[tokio::test]
async fn segmentation_idempotent() {
    run_idempotent(
        SegmentationPack,
        serde_json::json!({"records": [[1.0], [2.0], [10.0]], "k": 2}),
    )
    .await;
}

#[tokio::test]
async fn ranking_idempotent() {
    run_idempotent(
        RankingPack,
        serde_json::json!({
            "items": [{"id": "a", "scores": [1.0]}, {"id": "b", "scores": [2.0]}],
            "weights": [1.0],
            "higher_is_better": [true]
        }),
    )
    .await;
}

#[tokio::test]
async fn forecasting_idempotent() {
    run_idempotent(
        ForecastingPack,
        serde_json::json!({"values": [1.0, 2.0, 3.0, 4.0], "horizon": 2, "alpha": 0.3}),
    )
    .await;
}

#[tokio::test]
async fn classification_idempotent() {
    run_idempotent(
        ClassificationPack,
        serde_json::json!({
            "records": [[1.0], [-1.0]],
            "weights": [1.0],
            "bias": 0.0,
            "threshold": 0.5
        }),
    )
    .await;
}

#[tokio::test]
async fn regression_idempotent() {
    run_idempotent(
        RegressionPack,
        serde_json::json!({"records": [[1.0], [2.0]], "weights": [5.0], "bias": 1.0}),
    )
    .await;
}

#[tokio::test]
async fn similarity_idempotent() {
    run_idempotent(
        SimilarityPack,
        serde_json::json!({
            "items": [
                {"id": "a", "features": [1.0, 0.0]},
                {"id": "b", "features": [0.0, 1.0]}
            ],
            "metric": "cosine"
        }),
    )
    .await;
}

#[tokio::test]
async fn trend_detection_idempotent() {
    run_idempotent(
        TrendDetectionPack,
        serde_json::json!({"values": [1.0, 2.0, 3.0, 4.0, 5.0], "window": 3}),
    )
    .await;
}

#[tokio::test]
async fn descriptive_stats_idempotent() {
    run_idempotent(
        DescriptiveStatsPack,
        serde_json::json!({"values": [10.0, 20.0, 30.0]}),
    )
    .await;
}

// ── Constant data: anomaly detection handles zero std_dev gracefully ──

#[tokio::test]
async fn anomaly_detection_constant_data_produces_no_anomalies() {
    let mut engine = Engine::with_budget(budget());
    engine.register_suggestor(SolverSuggestor::new(
        AnomalyDetectionPack,
        ContextKey::Seeds,
        ContextKey::Strategies,
    ));
    let mut ctx = ContextState::new();
    let _ = ctx.add_input(
        ContextKey::Seeds,
        "input-1",
        serde_json::json!({"values": [5.0, 5.0, 5.0, 5.0, 5.0], "threshold": 2.0}).to_string(),
    );
    let result = engine.run(ctx).await.expect("should converge");
    assert!(result.converged);
    let strategies = result.context.get(ContextKey::Strategies);
    assert_eq!(strategies.len(), 1);
    // Zero std_dev → no anomalies detected
    assert!(strategies[0].content.contains("\"anomaly_count\":0"));
}

// ── Single-element edge cases ──

#[tokio::test]
async fn descriptive_stats_single_value() {
    let mut engine = Engine::with_budget(budget());
    engine.register_suggestor(SolverSuggestor::new(
        DescriptiveStatsPack,
        ContextKey::Seeds,
        ContextKey::Strategies,
    ));
    let mut ctx = ContextState::new();
    let _ = ctx.add_input(
        ContextKey::Seeds,
        "input-1",
        serde_json::json!({"values": [42.0]}).to_string(),
    );
    let result = engine.run(ctx).await.expect("should converge");
    assert!(result.converged);
    let content = &result.context.get(ContextKey::Strategies)[0].content;
    // Single value: mean = median = 42, std = 0
    assert!(content.contains("42"));
    assert!(content.contains("\"std_dev\":0"));
}
