//! Analytics Packs — each pack as a Suggestor in the convergence loop.
//!
//! Shows: anomaly detection, segmentation, ranking, forecasting, classification,
//! regression, similarity, trend detection, and descriptive stats — all converging
//! via the standard Engine.

use converge_kernel::{Budget, ContextKey, ContextState, Engine};
use converge_pack::PackSuggestor;

use converge_analytics::packs::{
    AnomalyDetectionPack, ClassificationPack, DescriptiveStatsPack, ForecastingPack, RankingPack,
    RegressionPack, SegmentationPack, SimilarityPack, TrendDetectionPack,
};

async fn run_pack(name: &str, input: serde_json::Value) {
    println!("--- {name} ---");

    let mut engine = Engine::with_budget(Budget {
        max_cycles: 5,
        max_facts: 100,
    });

    match name {
        "anomaly-detection" => {
            engine.register_suggestor(PackSuggestor::new(
                AnomalyDetectionPack,
                ContextKey::Seeds,
                ContextKey::Strategies,
            ));
        }
        "segmentation" => {
            engine.register_suggestor(PackSuggestor::new(
                SegmentationPack,
                ContextKey::Seeds,
                ContextKey::Strategies,
            ));
        }
        "ranking" => {
            engine.register_suggestor(PackSuggestor::new(
                RankingPack,
                ContextKey::Seeds,
                ContextKey::Strategies,
            ));
        }
        "forecasting" => {
            engine.register_suggestor(PackSuggestor::new(
                ForecastingPack,
                ContextKey::Seeds,
                ContextKey::Strategies,
            ));
        }
        "classification" => {
            engine.register_suggestor(PackSuggestor::new(
                ClassificationPack,
                ContextKey::Seeds,
                ContextKey::Strategies,
            ));
        }
        "regression" => {
            engine.register_suggestor(PackSuggestor::new(
                RegressionPack,
                ContextKey::Seeds,
                ContextKey::Strategies,
            ));
        }
        "similarity" => {
            engine.register_suggestor(PackSuggestor::new(
                SimilarityPack,
                ContextKey::Seeds,
                ContextKey::Strategies,
            ));
        }
        "trend-detection" => {
            engine.register_suggestor(PackSuggestor::new(
                TrendDetectionPack,
                ContextKey::Seeds,
                ContextKey::Strategies,
            ));
        }
        "descriptive-stats" => {
            engine.register_suggestor(PackSuggestor::new(
                DescriptiveStatsPack,
                ContextKey::Seeds,
                ContextKey::Strategies,
            ));
        }
        _ => panic!("Unknown pack: {name}"),
    }

    let mut ctx = ContextState::new();
    let _ = ctx.add_input(ContextKey::Seeds, "input-1", input.to_string());

    let result = engine.run(ctx).await.expect("should converge");

    println!(
        "  Converged: {} (cycles: {})",
        result.converged, result.cycles
    );
    for fact in result.context.get(ContextKey::Strategies) {
        let preview: String = fact.content.chars().take(120).collect();
        println!("  Output: {preview}...");
    }
    println!();
}

#[tokio::main]
async fn main() {
    println!("=== Analytics Packs Examples ===\n");

    run_pack(
        "anomaly-detection",
        serde_json::json!({
            "values": [10.0, 11.0, 10.5, 9.8, 10.2, 50.0, 10.1, 9.9, 10.3, 100.0],
            "threshold": 2.0
        }),
    )
    .await;

    run_pack(
        "segmentation",
        serde_json::json!({
            "records": [
                [1.0, 1.0], [1.1, 0.9], [0.9, 1.1],
                [5.0, 5.0], [5.1, 4.9], [4.9, 5.1],
                [9.0, 9.0], [9.1, 8.9], [8.9, 9.1]
            ],
            "k": 3
        }),
    )
    .await;

    run_pack(
        "ranking",
        serde_json::json!({
            "items": [
                {"id": "vendor-a", "scores": [0.9, 0.3, 50000.0]},
                {"id": "vendor-b", "scores": [0.7, 0.8, 30000.0]},
                {"id": "vendor-c", "scores": [0.5, 0.9, 20000.0]}
            ],
            "weights": [0.4, 0.4, 0.2],
            "higher_is_better": [true, true, false]
        }),
    )
    .await;

    run_pack(
        "forecasting",
        serde_json::json!({
            "values": [100.0, 110.0, 120.0, 115.0, 125.0, 130.0, 128.0, 135.0],
            "horizon": 3,
            "alpha": 0.4
        }),
    )
    .await;

    run_pack(
        "classification",
        serde_json::json!({
            "records": [[2.0, 3.0], [-1.0, -2.0], [1.5, 2.5], [-0.5, -1.5]],
            "weights": [1.0, 1.0],
            "bias": -2.0,
            "threshold": 0.5,
            "labels": ["high-value", "low-value"]
        }),
    )
    .await;

    run_pack(
        "regression",
        serde_json::json!({
            "records": [[1.0, 2.0], [2.0, 3.0], [3.0, 4.0], [4.0, 5.0]],
            "weights": [10.0, 5.0],
            "bias": 100.0
        }),
    )
    .await;

    run_pack(
        "similarity",
        serde_json::json!({
            "items": [
                {"id": "doc-a", "features": [1.0, 0.0, 1.0]},
                {"id": "doc-b", "features": [1.0, 0.1, 0.9]},
                {"id": "doc-c", "features": [0.0, 1.0, 0.0]},
                {"id": "doc-d", "features": [0.1, 0.9, 0.1]}
            ],
            "metric": "cosine",
            "top_k": 3
        }),
    )
    .await;

    run_pack(
        "trend-detection",
        serde_json::json!({
            "values": [10.0, 12.0, 14.0, 16.0, 15.0, 8.0, 6.0, 4.0, 5.0, 7.0, 9.0, 11.0],
            "window": 3,
            "sensitivity": 1.0
        }),
    )
    .await;

    run_pack(
        "descriptive-stats",
        serde_json::json!({
            "values": [23.0, 45.0, 12.0, 67.0, 34.0, 89.0, 56.0, 78.0, 11.0, 90.0],
            "percentiles": [25.0, 50.0, 75.0, 95.0]
        }),
    )
    .await;

    println!("=== Done ===");
}
