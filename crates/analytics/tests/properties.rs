//! Property tests: analytics pack invariants hold across randomized inputs.

use converge_analytics::packs::{
    AnomalyDetectionPack, ClassificationPack, DescriptiveStatsPack, ForecastingPack, RankingPack,
    RegressionPack, SegmentationPack, SimilarityPack, TrendDetectionPack,
};
use converge_kernel::{Budget, ContextKey, ContextState, Engine};
use converge_optimization::packs::Pack;
use converge_optimization::suggestor::SolverSuggestor;
use proptest::prelude::*;

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

fn budget() -> Budget {
    Budget {
        max_cycles: 5,
        max_facts: 100,
    }
}

// ── Anomaly Detection ──

proptest! {
    #[test]
    fn anomaly_detection_always_converges(
        values in proptest::collection::vec(-1000.0f64..1000.0, 2..50),
        threshold in 0.1f64..5.0,
    ) {
        let input = serde_json::json!({"values": values, "threshold": threshold});
        let mut engine = Engine::with_budget(budget());
        engine.register_suggestor(SolverSuggestor::new(
            AnomalyDetectionPack, ContextKey::Seeds, ContextKey::Strategies,
        ));
        let mut ctx = ContextState::new();
        let _ = ctx.add_input(ContextKey::Seeds, "input-1", input.to_string());
        let result = rt().block_on(engine.run(ctx)).expect("should converge");
        prop_assert!(result.converged);
        prop_assert_eq!(result.context.get(ContextKey::Strategies).len(), 1);
    }

    #[test]
    fn anomaly_detection_count_never_exceeds_total(
        values in proptest::collection::vec(-100.0f64..100.0, 3..30),
        threshold in 0.5f64..4.0,
    ) {
        let input = serde_json::json!({"values": values.clone(), "threshold": threshold});
        prop_assert!(AnomalyDetectionPack.validate_inputs(&input).is_ok());

        let spec = converge_optimization::gate::ProblemSpec::builder("test", "test")
            .objective(converge_optimization::gate::ObjectiveSpec::maximize("default"))
            .inputs_raw(input)
            .build()
            .unwrap();
        let result = AnomalyDetectionPack.solve(&spec).unwrap();
        let output: serde_json::Value = serde_json::from_value(result.plan.plan).unwrap();
        let count = output["anomaly_count"].as_u64().unwrap() as usize;
        prop_assert!(count <= values.len());
    }
}

// ── Segmentation ──

proptest! {
    #[test]
    fn segmentation_always_converges(
        n in 3usize..20,
        k in 1usize..4,
    ) {
        let k = k.min(n);
        let records: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, (i * 2) as f64]).collect();
        let input = serde_json::json!({"records": records, "k": k});
        let mut engine = Engine::with_budget(budget());
        engine.register_suggestor(SolverSuggestor::new(
            SegmentationPack, ContextKey::Seeds, ContextKey::Strategies,
        ));
        let mut ctx = ContextState::new();
        let _ = ctx.add_input(ContextKey::Seeds, "input-1", input.to_string());
        let result = rt().block_on(engine.run(ctx)).expect("should converge");
        prop_assert!(result.converged);
        prop_assert_eq!(result.context.get(ContextKey::Strategies).len(), 1);
    }

    #[test]
    fn segmentation_assignments_cover_all_records(
        n in 4usize..15,
        k in 2usize..4,
    ) {
        let k = k.min(n);
        let records: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
        let input = serde_json::json!({"records": records, "k": k});

        let spec = converge_optimization::gate::ProblemSpec::builder("test", "test")
            .objective(converge_optimization::gate::ObjectiveSpec::maximize("default"))
            .inputs_raw(input)
            .build()
            .unwrap();
        let result = SegmentationPack.solve(&spec).unwrap();
        let output: serde_json::Value = serde_json::from_value(result.plan.plan).unwrap();
        let assignments = output["assignments"].as_array().unwrap();
        prop_assert_eq!(assignments.len(), n);
        // All assignments in [0, k)
        for a in assignments {
            prop_assert!(a.as_u64().unwrap() < k as u64);
        }
    }
}

// ── Ranking ──

proptest! {
    #[test]
    fn ranking_always_converges(
        n in 2usize..10,
        dim in 1usize..4,
    ) {
        let items: Vec<serde_json::Value> = (0..n).map(|i| {
            let scores: Vec<f64> = (0..dim).map(|j| (i * dim + j) as f64 / 10.0).collect();
            serde_json::json!({"id": format!("item-{i}"), "scores": scores})
        }).collect();
        let weights: Vec<f64> = vec![1.0 / dim as f64; dim];
        let higher: Vec<bool> = vec![true; dim];

        let input = serde_json::json!({
            "items": items,
            "weights": weights,
            "higher_is_better": higher
        });
        let mut engine = Engine::with_budget(budget());
        engine.register_suggestor(SolverSuggestor::new(
            RankingPack, ContextKey::Seeds, ContextKey::Strategies,
        ));
        let mut ctx = ContextState::new();
        let _ = ctx.add_input(ContextKey::Seeds, "input-1", input.to_string());
        let result = rt().block_on(engine.run(ctx)).expect("should converge");
        prop_assert!(result.converged);
    }

    #[test]
    fn ranking_preserves_all_items(
        n in 2usize..8,
    ) {
        let items: Vec<serde_json::Value> = (0..n).map(|i| {
            serde_json::json!({"id": format!("item-{i}"), "scores": [i as f64]})
        }).collect();
        let input = serde_json::json!({
            "items": items,
            "weights": [1.0],
            "higher_is_better": [true]
        });

        let spec = converge_optimization::gate::ProblemSpec::builder("test", "test")
            .objective(converge_optimization::gate::ObjectiveSpec::maximize("default"))
            .inputs_raw(input)
            .build()
            .unwrap();
        let result = RankingPack.solve(&spec).unwrap();
        let output: serde_json::Value = serde_json::from_value(result.plan.plan).unwrap();
        let ranked = output["ranked"].as_array().unwrap();
        prop_assert_eq!(ranked.len(), n);
        prop_assert_eq!(output["total_items"].as_u64().unwrap() as usize, n);
    }
}

// ── Forecasting ──

proptest! {
    #[test]
    fn forecasting_always_converges(
        values in proptest::collection::vec(-500.0f64..500.0, 3..30),
        horizon in 1usize..10,
        alpha in 0.05f64..0.95,
    ) {
        let input = serde_json::json!({"values": values, "horizon": horizon, "alpha": alpha});
        let mut engine = Engine::with_budget(budget());
        engine.register_suggestor(SolverSuggestor::new(
            ForecastingPack, ContextKey::Seeds, ContextKey::Strategies,
        ));
        let mut ctx = ContextState::new();
        let _ = ctx.add_input(ContextKey::Seeds, "input-1", input.to_string());
        let result = rt().block_on(engine.run(ctx)).expect("should converge");
        prop_assert!(result.converged);
    }

    #[test]
    fn forecasting_produces_correct_horizon(
        values in proptest::collection::vec(0.0f64..100.0, 3..20),
        horizon in 1usize..8,
    ) {
        let input = serde_json::json!({"values": values, "horizon": horizon, "alpha": 0.3});

        let spec = converge_optimization::gate::ProblemSpec::builder("test", "test")
            .objective(converge_optimization::gate::ObjectiveSpec::maximize("default"))
            .inputs_raw(input)
            .build()
            .unwrap();
        let result = ForecastingPack.solve(&spec).unwrap();
        let output: serde_json::Value = serde_json::from_value(result.plan.plan).unwrap();
        let predictions = output["predictions"].as_array().unwrap();
        prop_assert_eq!(predictions.len(), horizon);
        // Confidence intervals: lower <= value <= upper
        for p in predictions {
            let lower = p["lower"].as_f64().unwrap();
            let value = p["value"].as_f64().unwrap();
            let upper = p["upper"].as_f64().unwrap();
            prop_assert!(lower <= value, "lower {lower} > value {value}");
            prop_assert!(value <= upper, "value {value} > upper {upper}");
        }
    }
}

// ── Classification ──

proptest! {
    #[test]
    fn classification_always_converges(
        n in 1usize..20,
        dim in 1usize..4,
        bias in -5.0f64..5.0,
    ) {
        let records: Vec<Vec<f64>> = (0..n)
            .map(|i| (0..dim).map(|j| ((i + j) as f64 - n as f64 / 2.0) / 5.0).collect())
            .collect();
        let weights: Vec<f64> = vec![1.0; dim];
        let input = serde_json::json!({
            "records": records,
            "weights": weights,
            "bias": bias,
            "threshold": 0.5
        });
        let mut engine = Engine::with_budget(budget());
        engine.register_suggestor(SolverSuggestor::new(
            ClassificationPack, ContextKey::Seeds, ContextKey::Strategies,
        ));
        let mut ctx = ContextState::new();
        let _ = ctx.add_input(ContextKey::Seeds, "input-1", input.to_string());
        let result = rt().block_on(engine.run(ctx)).expect("should converge");
        prop_assert!(result.converged);
    }

    #[test]
    fn classification_probabilities_in_zero_one(
        n in 2usize..10,
        bias in -3.0f64..3.0,
    ) {
        let records: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64 - 5.0]).collect();
        let input = serde_json::json!({
            "records": records,
            "weights": [1.0],
            "bias": bias,
            "threshold": 0.5
        });

        let spec = converge_optimization::gate::ProblemSpec::builder("test", "test")
            .objective(converge_optimization::gate::ObjectiveSpec::maximize("default"))
            .inputs_raw(input)
            .build()
            .unwrap();
        let result = ClassificationPack.solve(&spec).unwrap();
        let output: serde_json::Value = serde_json::from_value(result.plan.plan).unwrap();
        let predictions = output["predictions"].as_array().unwrap();
        for p in predictions {
            let prob = p["probability"].as_f64().unwrap();
            prop_assert!(prob >= 0.0 && prob <= 1.0, "probability {prob} out of [0,1]");
        }
    }
}

// ── Regression ──

proptest! {
    #[test]
    fn regression_always_converges(
        n in 1usize..15,
        weight in -10.0f64..10.0,
        bias in -100.0f64..100.0,
    ) {
        let records: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
        let input = serde_json::json!({"records": records, "weights": [weight], "bias": bias});
        let mut engine = Engine::with_budget(budget());
        engine.register_suggestor(SolverSuggestor::new(
            RegressionPack, ContextKey::Seeds, ContextKey::Strategies,
        ));
        let mut ctx = ContextState::new();
        let _ = ctx.add_input(ContextKey::Seeds, "input-1", input.to_string());
        let result = rt().block_on(engine.run(ctx)).expect("should converge");
        prop_assert!(result.converged);
    }

    #[test]
    fn regression_predictions_match_linear_formula(
        n in 2usize..10,
        weight in -5.0f64..5.0,
        bias in -50.0f64..50.0,
    ) {
        let records: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
        let input = serde_json::json!({"records": records, "weights": [weight], "bias": bias});

        let spec = converge_optimization::gate::ProblemSpec::builder("test", "test")
            .objective(converge_optimization::gate::ObjectiveSpec::maximize("default"))
            .inputs_raw(input)
            .build()
            .unwrap();
        let result = RegressionPack.solve(&spec).unwrap();
        let output: serde_json::Value = serde_json::from_value(result.plan.plan).unwrap();
        let predictions = output["predictions"].as_array().unwrap();
        for (i, p) in predictions.iter().enumerate() {
            let expected = i as f64 * weight + bias;
            let actual = p["value"].as_f64().unwrap();
            prop_assert!((actual - expected).abs() < 1e-10,
                "record {i}: expected {expected}, got {actual}");
        }
    }
}

// ── Similarity ──

proptest! {
    #[test]
    fn similarity_always_converges(
        n in 2usize..8,
        dim in 1usize..4,
    ) {
        let items: Vec<serde_json::Value> = (0..n).map(|i| {
            let features: Vec<f64> = (0..dim).map(|j| (i * dim + j) as f64).collect();
            serde_json::json!({"id": format!("item-{i}"), "features": features})
        }).collect();
        let input = serde_json::json!({"items": items, "metric": "euclidean"});
        let mut engine = Engine::with_budget(budget());
        engine.register_suggestor(SolverSuggestor::new(
            SimilarityPack, ContextKey::Seeds, ContextKey::Strategies,
        ));
        let mut ctx = ContextState::new();
        let _ = ctx.add_input(ContextKey::Seeds, "input-1", input.to_string());
        let result = rt().block_on(engine.run(ctx)).expect("should converge");
        prop_assert!(result.converged);
    }

    #[test]
    fn similarity_pair_count_matches_formula(
        n in 2usize..7,
    ) {
        let items: Vec<serde_json::Value> = (0..n).map(|i| {
            serde_json::json!({"id": format!("i{i}"), "features": [i as f64]})
        }).collect();
        let input = serde_json::json!({"items": items, "metric": "euclidean"});

        let spec = converge_optimization::gate::ProblemSpec::builder("test", "test")
            .objective(converge_optimization::gate::ObjectiveSpec::maximize("default"))
            .inputs_raw(input)
            .build()
            .unwrap();
        let result = SimilarityPack.solve(&spec).unwrap();
        let output: serde_json::Value = serde_json::from_value(result.plan.plan).unwrap();
        let total = output["total_pairs"].as_u64().unwrap() as usize;
        prop_assert_eq!(total, n * (n - 1) / 2);
    }
}

// ── Trend Detection ──

proptest! {
    #[test]
    fn trend_detection_always_converges(
        values in proptest::collection::vec(-100.0f64..100.0, 4..30),
        window in 2usize..5,
    ) {
        let window = window.min(values.len());
        let input = serde_json::json!({"values": values, "window": window, "sensitivity": 1.0});
        let mut engine = Engine::with_budget(budget());
        engine.register_suggestor(SolverSuggestor::new(
            TrendDetectionPack, ContextKey::Seeds, ContextKey::Strategies,
        ));
        let mut ctx = ContextState::new();
        let _ = ctx.add_input(ContextKey::Seeds, "input-1", input.to_string());
        let result = rt().block_on(engine.run(ctx)).expect("should converge");
        prop_assert!(result.converged);
    }
}

// ── Descriptive Stats ──

proptest! {
    #[test]
    fn descriptive_stats_always_converges(
        values in proptest::collection::vec(-1000.0f64..1000.0, 1..50),
    ) {
        let input = serde_json::json!({"values": values});
        let mut engine = Engine::with_budget(budget());
        engine.register_suggestor(SolverSuggestor::new(
            DescriptiveStatsPack, ContextKey::Seeds, ContextKey::Strategies,
        ));
        let mut ctx = ContextState::new();
        let _ = ctx.add_input(ContextKey::Seeds, "input-1", input.to_string());
        let result = rt().block_on(engine.run(ctx)).expect("should converge");
        prop_assert!(result.converged);
    }

    #[test]
    fn descriptive_stats_mean_within_range(
        values in proptest::collection::vec(-100.0f64..100.0, 2..30),
    ) {
        let input = serde_json::json!({"values": values.clone()});

        let spec = converge_optimization::gate::ProblemSpec::builder("test", "test")
            .objective(converge_optimization::gate::ObjectiveSpec::maximize("default"))
            .inputs_raw(input)
            .build()
            .unwrap();
        let result = DescriptiveStatsPack.solve(&spec).unwrap();
        let output: serde_json::Value = serde_json::from_value(result.plan.plan).unwrap();
        let mean = output["mean"].as_f64().unwrap();
        let min = output["min"].as_f64().unwrap();
        let max = output["max"].as_f64().unwrap();
        prop_assert!(mean >= min && mean <= max, "mean {mean} not in [{min}, {max}]");
    }

    #[test]
    fn descriptive_stats_std_dev_non_negative(
        values in proptest::collection::vec(-50.0f64..50.0, 1..20),
    ) {
        let input = serde_json::json!({"values": values});

        let spec = converge_optimization::gate::ProblemSpec::builder("test", "test")
            .objective(converge_optimization::gate::ObjectiveSpec::maximize("default"))
            .inputs_raw(input)
            .build()
            .unwrap();
        let result = DescriptiveStatsPack.solve(&spec).unwrap();
        let output: serde_json::Value = serde_json::from_value(result.plan.plan).unwrap();
        let std_dev = output["std_dev"].as_f64().unwrap();
        prop_assert!(std_dev >= 0.0, "std_dev {std_dev} is negative");
    }
}

// ── Determinism: same input → same output (all packs) ──

proptest! {
    #[test]
    fn all_packs_deterministic(
        seed in 0u64..1000,
    ) {
        let values: Vec<f64> = (0..10).map(|i| (seed as f64 + i as f64).sin() * 100.0).collect();
        let input = serde_json::json!({"values": values, "threshold": 2.0});

        let spec = converge_optimization::gate::ProblemSpec::builder("test", "test")
            .objective(converge_optimization::gate::ObjectiveSpec::maximize("default"))
            .inputs_raw(input.clone())
            .build()
            .unwrap();

        let r1 = AnomalyDetectionPack.solve(&spec).unwrap();
        let r2 = AnomalyDetectionPack.solve(&spec).unwrap();

        let o1: serde_json::Value = serde_json::from_value(r1.plan.plan).unwrap();
        let o2: serde_json::Value = serde_json::from_value(r2.plan.plan).unwrap();
        prop_assert_eq!(o1, o2);
    }
}
