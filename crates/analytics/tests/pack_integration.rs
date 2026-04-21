//! Integration tests: each analytics pack as a Suggestor in the convergence loop.

use converge_analytics::packs::{
    AnomalyDetectionPack, ClassificationPack, DescriptiveStatsPack, ForecastingPack, RankingPack,
    RegressionPack, SegmentationPack, SimilarityPack, TrendDetectionPack,
};
use converge_kernel::{Budget, ContextKey, ContextState, ConvergeResult, Engine};
use converge_pack::PackSuggestor;

fn budget() -> Budget {
    Budget {
        max_cycles: 5,
        max_facts: 100,
    }
}

async fn run_with_input<P: converge_pack::Pack + 'static>(
    pack: P,
    input: serde_json::Value,
) -> ConvergeResult {
    let mut engine = Engine::with_budget(budget());
    engine.register_suggestor(PackSuggestor::new(
        pack,
        ContextKey::Seeds,
        ContextKey::Strategies,
    ));
    let mut ctx = ContextState::new();
    let _ = ctx.add_input(ContextKey::Seeds, "input-1", input.to_string());
    engine.run(ctx).await.expect("should converge")
}

#[tokio::test]
async fn anomaly_detection_finds_outliers() {
    let result = run_with_input(
        AnomalyDetectionPack,
        serde_json::json!({
            "values": [10.0, 11.0, 10.5, 9.8, 10.2, 50.0, 10.1, 9.9, 10.3, 100.0],
            "threshold": 2.0
        }),
    )
    .await;

    assert!(result.converged);
    let strategies = result.context.get(ContextKey::Strategies);
    assert_eq!(strategies.len(), 1);
    let content = &strategies[0].content;
    assert!(content.contains("anomalies"));
    assert!(content.contains("50.0") || content.contains("100.0"));
}

#[tokio::test]
async fn segmentation_clusters_distinct_groups() {
    let result = run_with_input(
        SegmentationPack,
        serde_json::json!({
            "records": [
                [1.0, 1.0], [1.1, 0.9], [0.9, 1.1],
                [10.0, 10.0], [10.1, 9.9], [9.9, 10.1]
            ],
            "k": 2
        }),
    )
    .await;

    assert!(result.converged);
    let strategies = result.context.get(ContextKey::Strategies);
    assert_eq!(strategies.len(), 1);
    let content = &strategies[0].content;
    assert!(content.contains("assignments"));
    assert!(content.contains("centroids"));
}

#[tokio::test]
async fn ranking_orders_by_composite_score() {
    let result = run_with_input(
        RankingPack,
        serde_json::json!({
            "items": [
                {"id": "worst", "scores": [0.1, 0.1]},
                {"id": "best", "scores": [0.9, 0.9]},
                {"id": "middle", "scores": [0.5, 0.5]}
            ],
            "weights": [0.5, 0.5],
            "higher_is_better": [true, true]
        }),
    )
    .await;

    assert!(result.converged);
    let strategies = result.context.get(ContextKey::Strategies);
    assert_eq!(strategies.len(), 1);
    let content = &strategies[0].content;
    // "best" should appear before "worst" in ranked output
    let best_pos = content.find("best").expect("best should be in output");
    let worst_pos = content.find("worst").expect("worst should be in output");
    assert!(best_pos < worst_pos, "best should rank before worst");
}

#[tokio::test]
async fn forecasting_produces_predictions() {
    let result = run_with_input(
        ForecastingPack,
        serde_json::json!({
            "values": [100.0, 110.0, 120.0, 130.0, 140.0],
            "horizon": 3,
            "alpha": 0.3
        }),
    )
    .await;

    assert!(result.converged);
    let strategies = result.context.get(ContextKey::Strategies);
    assert_eq!(strategies.len(), 1);
    let content = &strategies[0].content;
    assert!(content.contains("predictions"));
    assert!(content.contains("\"step\":1"));
    assert!(content.contains("\"step\":2"));
    assert!(content.contains("\"step\":3"));
}

#[tokio::test]
async fn classification_assigns_labels() {
    let result = run_with_input(
        ClassificationPack,
        serde_json::json!({
            "records": [[3.0, 3.0], [-3.0, -3.0], [2.0, 2.0], [-2.0, -2.0]],
            "weights": [1.0, 1.0],
            "bias": -2.0,
            "threshold": 0.5,
            "labels": ["spam", "not-spam"]
        }),
    )
    .await;

    assert!(result.converged);
    let strategies = result.context.get(ContextKey::Strategies);
    assert_eq!(strategies.len(), 1);
    let content = &strategies[0].content;
    assert!(content.contains("spam"));
    assert!(content.contains("not-spam"));
    assert!(content.contains("probability"));
}

#[tokio::test]
async fn regression_predicts_values() {
    let result = run_with_input(
        RegressionPack,
        serde_json::json!({
            "records": [[1.0], [2.0], [3.0], [4.0]],
            "weights": [10.0],
            "bias": 5.0
        }),
    )
    .await;

    assert!(result.converged);
    let strategies = result.context.get(ContextKey::Strategies);
    assert_eq!(strategies.len(), 1);
    let content = &strategies[0].content;
    // record [1.0] * 10.0 + 5.0 = 15.0
    assert!(content.contains("15"));
    // record [4.0] * 10.0 + 5.0 = 45.0
    assert!(content.contains("45"));
}

#[tokio::test]
async fn similarity_finds_nearest_pairs() {
    let result = run_with_input(
        SimilarityPack,
        serde_json::json!({
            "items": [
                {"id": "a", "features": [1.0, 0.0, 0.0]},
                {"id": "b", "features": [0.9, 0.1, 0.0]},
                {"id": "c", "features": [0.0, 0.0, 1.0]}
            ],
            "metric": "cosine",
            "top_k": 2
        }),
    )
    .await;

    assert!(result.converged);
    let strategies = result.context.get(ContextKey::Strategies);
    assert_eq!(strategies.len(), 1);
    let content = &strategies[0].content;
    assert!(content.contains("pairs"));
    // a and b should be most similar (both point roughly in same direction)
    let a_b = content
        .find("\"id_a\":\"a\"")
        .or_else(|| content.find("\"id_b\":\"a\""));
    assert!(a_b.is_some(), "pair involving 'a' should exist");
}

#[tokio::test]
async fn trend_detection_identifies_segments() {
    let result = run_with_input(
        TrendDetectionPack,
        serde_json::json!({
            "values": [1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0, 1.0],
            "window": 3,
            "sensitivity": 1.0
        }),
    )
    .await;

    assert!(result.converged);
    let strategies = result.context.get(ContextKey::Strategies);
    assert_eq!(strategies.len(), 1);
    let content = &strategies[0].content;
    assert!(content.contains("segments"));
    assert!(content.contains("rising") || content.contains("falling"));
}

#[tokio::test]
async fn descriptive_stats_computes_summary() {
    let result = run_with_input(
        DescriptiveStatsPack,
        serde_json::json!({
            "values": [10.0, 20.0, 30.0, 40.0, 50.0],
            "percentiles": [25.0, 50.0, 75.0]
        }),
    )
    .await;

    assert!(result.converged);
    let strategies = result.context.get(ContextKey::Strategies);
    assert_eq!(strategies.len(), 1);
    let content = &strategies[0].content;
    assert!(content.contains("mean"));
    assert!(content.contains("median"));
    assert!(content.contains("std_dev"));
    // mean of [10,20,30,40,50] = 30
    assert!(content.contains("30"));
}
