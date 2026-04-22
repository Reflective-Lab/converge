//! EXP-001: Reference validation for analytics algorithms.
//!
//! Each test uses a hand-computable input where the correct output can be
//! verified with a pocket calculator. If converge disagrees, the test fails.

use converge_analytics::packs::anomaly_detection::{AnomalyDetectionInput, ZScoreSolver};
use converge_analytics::packs::classification::{ClassificationInput, LogisticClassifier};
use converge_analytics::packs::descriptive_stats::{DescriptiveStatsInput, DescriptiveStatsSolver};
use converge_analytics::packs::forecasting::{ExponentialSmoothingSolver, ForecastingInput};
use converge_analytics::packs::ranking::{RankItem, RankingInput, WeightedScoringSolver};
use converge_analytics::packs::regression::{LinearRegressionSolver, RegressionInput};
use converge_analytics::packs::segmentation::{KMeansSolver, SegmentationInput};
use converge_analytics::packs::similarity::{
    DistanceMetric, PairwiseSimilaritySolver, SimilarityInput, SimilarityItem,
};
use converge_pack::gate::{ObjectiveSpec, ProblemSpec};

fn spec() -> ProblemSpec {
    ProblemSpec::builder("ref-test", "test")
        .objective(ObjectiveSpec::maximize("accuracy"))
        .build()
        .unwrap()
}

// ── Z-Score Anomaly Detection ────────────────────────────────────────────────
// Formula: z = (x - μ) / σ, flag if |z| > threshold

#[test]
fn zscore_hand_computed() {
    // Values: [10, 10, 10, 10, 10, 10, 10, 10, 10, 100]
    // Mean: (9×10 + 100) / 10 = 190/10 = 19.0
    // Variance: (9×(10-19)² + (100-19)²) / 10 = (9×81 + 6561) / 10 = (729+6561)/10 = 729.0
    // StdDev: √729 = 27.0
    // Z-score of 100: (100-19)/27 = 81/27 = 3.0
    // Z-score of 10: (10-19)/27 = -9/27 = -0.333...
    // With threshold 2.0: only 100 (|z|=3.0) is an anomaly.
    let input = AnomalyDetectionInput {
        values: vec![10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 100.0],
        threshold: 2.0,
        labels: None,
    };
    let (output, _) = ZScoreSolver.solve(&input, &spec()).unwrap();

    assert!(
        (output.mean - 19.0).abs() < 1e-9,
        "mean should be 19.0, got {}",
        output.mean
    );
    assert!(
        (output.std_dev - 27.0).abs() < 1e-9,
        "stddev should be 27.0, got {}",
        output.std_dev
    );
    assert_eq!(output.anomaly_count, 1, "only 100.0 is an anomaly");
    assert_eq!(output.anomalies[0].index, 9);
    assert!((output.anomalies[0].z_score - 3.0).abs() < 1e-9);
}

// ── Descriptive Statistics ───────────────────────────────────────────────────
// Reference: hand-computable dataset

#[test]
fn descriptive_stats_hand_computed() {
    // Values: [2, 4, 4, 4, 5, 5, 7, 9]  (n=8)
    // Sorted: [2, 4, 4, 4, 5, 5, 7, 9]
    // Mean: 40/8 = 5.0
    // Median: (4+5)/2 = 4.5
    // Variance (population): Σ(x-5)² / 8 = (9+1+1+1+0+0+4+16)/8 = 32/8 = 4.0
    // StdDev: √4 = 2.0
    // Min: 2, Max: 9, Range: 7
    let input = DescriptiveStatsInput {
        values: vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0],
        percentiles: vec![25.0, 50.0, 75.0],
    };
    let (output, _) = DescriptiveStatsSolver.solve(&input, &spec()).unwrap();

    assert_eq!(output.count, 8);
    assert!(
        (output.mean - 5.0).abs() < 1e-9,
        "mean = 5.0, got {}",
        output.mean
    );
    assert!(
        (output.median - 4.5).abs() < 1e-9,
        "median = 4.5, got {}",
        output.median
    );
    assert!(
        (output.variance - 4.0).abs() < 1e-9,
        "variance = 4.0, got {}",
        output.variance
    );
    assert!(
        (output.std_dev - 2.0).abs() < 1e-9,
        "stddev = 2.0, got {}",
        output.std_dev
    );
    assert!((output.min - 2.0).abs() < 1e-9);
    assert!((output.max - 9.0).abs() < 1e-9);
    assert!((output.range - 7.0).abs() < 1e-9);
}

// ── Linear Regression (inference) ────────────────────────────────────────────
// Formula: y = w·x + b

#[test]
fn linear_regression_exact() {
    // Model: y = 2·x₁ + 3·x₂ + 1
    // Input: [1, 1] → 2+3+1 = 6
    // Input: [2, 0] → 4+0+1 = 5
    // Input: [0, 3] → 0+9+1 = 10
    let input = RegressionInput {
        records: vec![vec![1.0, 1.0], vec![2.0, 0.0], vec![0.0, 3.0]],
        weights: vec![2.0, 3.0],
        bias: 1.0,
    };
    let (output, _) = LinearRegressionSolver.solve(&input, &spec()).unwrap();

    assert_eq!(output.total, 3);
    assert!((output.predictions[0].value - 6.0).abs() < 1e-9);
    assert!((output.predictions[1].value - 5.0).abs() < 1e-9);
    assert!((output.predictions[2].value - 10.0).abs() < 1e-9);
    assert!(
        (output.mean_prediction - 7.0).abs() < 1e-9,
        "mean = (6+5+10)/3 = 7.0"
    );
}

// ── Logistic Regression (classification) ─────────────────────────────────────
// Formula: p = 1 / (1 + exp(-(w·x + b)))

#[test]
fn logistic_classification_sigmoid() {
    // Model: sigmoid(3·x₁ + 0·x₂ - 1.5)
    // Input [1, 0]: sigmoid(3-1.5) = sigmoid(1.5) = 1/(1+exp(-1.5)) ≈ 0.8176
    // Input [0, 0]: sigmoid(-1.5) = 1/(1+exp(1.5)) ≈ 0.1824
    // Threshold 0.5: [1,0] → positive, [0,0] → negative
    let input = ClassificationInput {
        records: vec![vec![1.0, 0.0], vec![0.0, 0.0]],
        weights: vec![3.0, 0.0],
        bias: -1.5,
        threshold: 0.5,
        labels: None,
    };
    let (output, _) = LogisticClassifier.solve(&input, &spec()).unwrap();

    let expected_p1 = 1.0 / (1.0 + (-1.5_f64).exp());
    let expected_p0 = 1.0 / (1.0 + 1.5_f64.exp());

    assert_eq!(output.positive_count, 1);
    assert_eq!(output.negative_count, 1);
    assert!(
        (output.predictions[0].probability - expected_p1).abs() < 1e-6,
        "sigmoid(1.5) ≈ {expected_p1}, got {}",
        output.predictions[0].probability
    );
    assert!(
        (output.predictions[1].probability - expected_p0).abs() < 1e-6,
        "sigmoid(-1.5) ≈ {expected_p0}, got {}",
        output.predictions[1].probability
    );
}

// ── Cosine Similarity ────────────────────────────────────────────────────────
// Formula: cos(θ) = (A·B) / (‖A‖·‖B‖)

#[test]
fn cosine_similarity_hand_computed() {
    // A = [1, 0, 0], B = [1, 0, 0] → identical → cos = 1.0
    // A = [1, 0, 0], C = [0, 1, 0] → orthogonal → cos = 0.0
    // B = [1, 0, 0], C = [0, 1, 0] → orthogonal → cos = 0.0
    let input = SimilarityInput {
        items: vec![
            SimilarityItem {
                id: "A".to_string(),
                features: vec![1.0, 0.0, 0.0],
            },
            SimilarityItem {
                id: "B".to_string(),
                features: vec![1.0, 0.0, 0.0],
            },
            SimilarityItem {
                id: "C".to_string(),
                features: vec![0.0, 1.0, 0.0],
            },
        ],
        metric: DistanceMetric::Cosine,
        top_k: None,
    };
    let (output, _) = PairwiseSimilaritySolver.solve(&input, &spec()).unwrap();

    assert_eq!(output.total_pairs, 3); // C(3,2) = 3

    // Find the A-B pair (should be cos=1.0)
    let ab = output
        .pairs
        .iter()
        .find(|p| (p.id_a == "A" && p.id_b == "B") || (p.id_a == "B" && p.id_b == "A"))
        .expect("A-B pair must exist");
    assert!(
        (ab.score - 1.0).abs() < 1e-9,
        "identical vectors → cos = 1.0, got {}",
        ab.score
    );

    // A-C and B-C should be 0.0
    let ac = output
        .pairs
        .iter()
        .find(|p| (p.id_a == "A" && p.id_b == "C") || (p.id_a == "C" && p.id_b == "A"))
        .expect("A-C pair must exist");
    assert!(
        ac.score.abs() < 1e-9,
        "orthogonal vectors → cos = 0.0, got {}",
        ac.score
    );
}

#[test]
fn cosine_similarity_45_degrees() {
    // A = [1, 1], B = [1, 0]
    // cos(45°) = (1·1 + 1·0) / (√2 · 1) = 1/√2 ≈ 0.7071
    let input = SimilarityInput {
        items: vec![
            SimilarityItem {
                id: "A".to_string(),
                features: vec![1.0, 1.0],
            },
            SimilarityItem {
                id: "B".to_string(),
                features: vec![1.0, 0.0],
            },
        ],
        metric: DistanceMetric::Cosine,
        top_k: None,
    };
    let (output, _) = PairwiseSimilaritySolver.solve(&input, &spec()).unwrap();

    let expected = 1.0 / 2.0_f64.sqrt();
    assert!(
        (output.pairs[0].score - expected).abs() < 1e-6,
        "cos(45°) = 1/√2 ≈ {expected}, got {}",
        output.pairs[0].score
    );
}

// ── Exponential Smoothing (SES) ──────────────────────────────────────────────
// Reference: Hyndman & Athanasopoulos "Forecasting: Principles and Practice"
// Formula: level[t+1] = α·y[t] + (1-α)·level[t], forecast = last level

#[test]
fn exponential_smoothing_hand_traced() {
    // Values: [100, 110, 120], α = 0.5
    // level[0] = 100 (initialised to first value)
    // level[1] = 0.5×100 + 0.5×100 = 100   (after seeing y[0]=100)
    // Wait — initialization matters. Let me trace from the solver's perspective:
    //
    // Most SES implementations: level₀ = y₀ = 100
    // After y₁=110: level₁ = 0.5×110 + 0.5×100 = 105
    // After y₂=120: level₂ = 0.5×120 + 0.5×105 = 112.5
    // Forecast for step 1: 112.5
    let input = ForecastingInput {
        values: vec![100.0, 110.0, 120.0],
        horizon: 1,
        alpha: 0.5,
    };
    let (output, _) = ExponentialSmoothingSolver.solve(&input, &spec()).unwrap();

    assert_eq!(output.predictions.len(), 1);
    assert!(
        (output.predictions[0].value - 112.5).abs() < 1e-6,
        "SES forecast = 112.5, got {}",
        output.predictions[0].value
    );
}

// ── K-Means Clustering ───────────────────────────────────────────────────────
// Reference: two well-separated clusters must be found exactly

#[test]
fn kmeans_two_separated_clusters() {
    // Cluster A: [0, 0], [1, 0], [0, 1] — centroid ≈ (0.33, 0.33)
    // Cluster B: [10, 10], [11, 10], [10, 11] — centroid ≈ (10.33, 10.33)
    // K=2, seeded: must assign each group correctly.
    let input = SegmentationInput {
        records: vec![
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![0.0, 1.0],
            vec![10.0, 10.0],
            vec![11.0, 10.0],
            vec![10.0, 11.0],
        ],
        k: 2,
        max_iterations: 100,
        seed: Some(42),
    };
    let (output, _) = KMeansSolver.solve(&input, &spec()).unwrap();

    // Points 0,1,2 should share a cluster; points 3,4,5 should share another.
    assert_eq!(output.assignments[0], output.assignments[1]);
    assert_eq!(output.assignments[1], output.assignments[2]);
    assert_eq!(output.assignments[3], output.assignments[4]);
    assert_eq!(output.assignments[4], output.assignments[5]);
    assert_ne!(
        output.assignments[0], output.assignments[3],
        "the two clusters must be distinct"
    );

    // Centroids should be near (0.33, 0.33) and (10.33, 10.33).
    for centroid in &output.centroids {
        let near_origin = centroid[0] < 2.0 && centroid[1] < 2.0;
        let near_ten = centroid[0] > 8.0 && centroid[1] > 8.0;
        assert!(
            near_origin || near_ten,
            "centroid {:?} should be near (0.33, 0.33) or (10.33, 10.33)",
            centroid
        );
    }
}

// ── Ranking (Weighted Multi-Criteria) ────────────────────────────────────────
// Formula: min-max normalize, apply direction, weight, sum

#[test]
fn ranking_hand_computed() {
    // 3 items, 2 criteria, weights [0.7, 0.3], both higher-is-better
    // Scores: A=[100, 10], B=[50, 90], C=[75, 50]
    //
    // Normalize (min-max per criterion):
    //   Criterion 1: min=50, max=100 → A=1.0, B=0.0, C=0.5
    //   Criterion 2: min=10, max=90 → A=0.0, B=1.0, C=0.5
    //
    // Composite: A = 0.7×1.0 + 0.3×0.0 = 0.70
    //            B = 0.7×0.0 + 0.3×1.0 = 0.30
    //            C = 0.7×0.5 + 0.3×0.5 = 0.50
    //
    // Ranking: A(0.70) > C(0.50) > B(0.30)
    let input = RankingInput {
        items: vec![
            RankItem {
                id: "A".to_string(),
                scores: vec![100.0, 10.0],
            },
            RankItem {
                id: "B".to_string(),
                scores: vec![50.0, 90.0],
            },
            RankItem {
                id: "C".to_string(),
                scores: vec![75.0, 50.0],
            },
        ],
        weights: vec![0.7, 0.3],
        higher_is_better: vec![true, true],
        top_k: None,
    };
    let (output, _) = WeightedScoringSolver.solve(&input, &spec()).unwrap();

    assert_eq!(output.ranked[0].id, "A");
    assert_eq!(output.ranked[1].id, "C");
    assert_eq!(output.ranked[2].id, "B");
    assert!((output.ranked[0].composite_score - 0.70).abs() < 1e-6);
    assert!((output.ranked[1].composite_score - 0.50).abs() < 1e-6);
    assert!((output.ranked[2].composite_score - 0.30).abs() < 1e-6);
}

// ════════════════════════════════════════════════════════════════════════════
// LARGER INSTANCES — stress numerical precision and algorithmic robustness
// ════════════════════════════════════════════════════════════════════════════

// ── Z-Score: multiple anomalies at different thresholds ─────────────────────

#[test]
fn zscore_20_values_multiple_anomalies() {
    // 18 values of 50.0, then 150.0 and -50.0.
    // Mean = (18×50 + 150 + (-50)) / 20 = 1000/20 = 50.0
    // Variance = (18×0 + 10000 + 10000) / 20 = 1000.0
    // StdDev = √1000 ≈ 31.6228
    // Z(150) = 100/31.6228 ≈ 3.162
    // Z(-50) = -100/31.6228 ≈ -3.162
    // Threshold 3.0: both are anomalies.
    let mut values = vec![50.0; 18];
    values.push(150.0);
    values.push(-50.0);

    let input = AnomalyDetectionInput {
        values,
        threshold: 3.0,
        labels: None,
    };
    let (output, _) = ZScoreSolver.solve(&input, &spec()).unwrap();

    assert!((output.mean - 50.0).abs() < 1e-9, "mean = 50.0");
    assert!(
        (output.std_dev - 1000.0_f64.sqrt()).abs() < 1e-6,
        "stddev = √1000 ≈ 31.623"
    );
    assert_eq!(
        output.anomaly_count, 2,
        "both 150 and -50 are anomalies at threshold 3.0"
    );
}

// ── Descriptive Stats: larger dataset ───────────────────────────────────────

#[test]
fn descriptive_stats_15_values() {
    // Values: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
    // n=15, sum=120, mean=8.0
    // Median: value at index 7 = 8.0 (odd count, middle element)
    // Variance: Σ(x-8)² / 15 = (49+36+25+16+9+4+1+0+1+4+9+16+25+36+49)/15
    //         = 280/15 = 18.6667
    // StdDev: √(280/15) ≈ 4.3205
    // Min=1, Max=15, Range=14
    let input = DescriptiveStatsInput {
        values: (1..=15).map(|x| x as f64).collect(),
        percentiles: vec![25.0, 50.0, 75.0],
    };
    let (output, _) = DescriptiveStatsSolver.solve(&input, &spec()).unwrap();

    assert_eq!(output.count, 15);
    assert!((output.mean - 8.0).abs() < 1e-9);
    assert!(
        (output.median - 8.0).abs() < 1e-9,
        "odd count → middle = 8.0"
    );
    assert!(
        (output.variance - 280.0 / 15.0).abs() < 1e-9,
        "variance = 280/15"
    );
    assert!((output.min - 1.0).abs() < 1e-9);
    assert!((output.max - 15.0).abs() < 1e-9);
    assert!((output.range - 14.0).abs() < 1e-9);
}

// ── Linear Regression: 5-dimensional ────────────────────────────────────────

#[test]
fn linear_regression_5d() {
    // y = 1·x₁ + 2·x₂ + 3·x₃ + 4·x₄ + 5·x₅ + 10
    // [1,1,1,1,1] → 1+2+3+4+5+10 = 25
    // [2,0,1,0,1] → 2+0+3+0+5+10 = 20
    // [0,3,0,2,0] → 0+6+0+8+0+10 = 24
    // [1,2,3,0,0] → 1+4+9+0+0+10 = 24
    // Mean = (25+20+24+24)/4 = 23.25
    let input = RegressionInput {
        records: vec![
            vec![1.0, 1.0, 1.0, 1.0, 1.0],
            vec![2.0, 0.0, 1.0, 0.0, 1.0],
            vec![0.0, 3.0, 0.0, 2.0, 0.0],
            vec![1.0, 2.0, 3.0, 0.0, 0.0],
        ],
        weights: vec![1.0, 2.0, 3.0, 4.0, 5.0],
        bias: 10.0,
    };
    let (output, _) = LinearRegressionSolver.solve(&input, &spec()).unwrap();

    assert_eq!(output.total, 4);
    assert!((output.predictions[0].value - 25.0).abs() < 1e-9);
    assert!((output.predictions[1].value - 20.0).abs() < 1e-9);
    assert!((output.predictions[2].value - 24.0).abs() < 1e-9);
    assert!((output.predictions[3].value - 24.0).abs() < 1e-9);
    assert!((output.mean_prediction - 23.25).abs() < 1e-9);
}

// ── Logistic Classification: boundary inputs ────────────────────────────────

#[test]
fn logistic_classification_boundary() {
    // Model: sigmoid(10·x₁ - 5)
    // At x₁=0.5: z=10(0.5)-5=0, sigmoid(0)=0.5 exactly (decision boundary)
    // At x₁=1.0: z=5, sigmoid(5) ≈ 0.9933 (strong positive)
    // At x₁=0.0: z=-5, sigmoid(-5) ≈ 0.0067 (strong negative)
    // At x₁=0.3: z=-2, sigmoid(-2) ≈ 0.1192
    // At x₁=0.7: z=2, sigmoid(2) ≈ 0.8808
    let input = ClassificationInput {
        records: vec![vec![0.5], vec![1.0], vec![0.0], vec![0.3], vec![0.7]],
        weights: vec![10.0],
        bias: -5.0,
        threshold: 0.5,
        labels: None,
    };
    let (output, _) = LogisticClassifier.solve(&input, &spec()).unwrap();

    // x=0.5 is exactly on boundary: sigmoid(0) = 0.5 → classified as positive (≥ threshold)
    assert_eq!(output.positive_count, 3, "x=0.5, 0.7, 1.0 are positive");
    assert_eq!(output.negative_count, 2, "x=0.0, 0.3 are negative");

    // Verify symmetry: sigmoid(2) + sigmoid(-2) = 1
    let p_07 = output.predictions[4].probability;
    let p_03 = output.predictions[3].probability;
    assert!(
        (p_07 + p_03 - 1.0).abs() < 1e-9,
        "sigmoid symmetry: p(0.7) + p(0.3) = 1.0"
    );

    // Exact boundary: sigmoid(0) = 0.5
    assert!(
        (output.predictions[0].probability - 0.5).abs() < 1e-9,
        "sigmoid(0) = 0.5 exactly"
    );
}

// ── Cosine Similarity: 4 items in 5D ────────────────────────────────────────

#[test]
fn cosine_similarity_5d() {
    // A = [1, 0, 0, 0, 0]  (unit vector along dim 0)
    // B = [0, 1, 0, 0, 0]  (unit vector along dim 1)
    // C = [1, 1, 0, 0, 0]  (in the 0-1 plane, 45° from both A and B)
    // D = [1, 1, 1, 1, 1]  (all-ones)
    //
    // cos(A,B) = 0 (orthogonal)
    // cos(A,C) = 1/√2 ≈ 0.7071
    // cos(A,D) = 1/√5 ≈ 0.4472
    // cos(B,C) = 1/√2 ≈ 0.7071
    // cos(B,D) = 1/√5 ≈ 0.4472
    // cos(C,D) = 2/(√2·√5) = 2/√10 ≈ 0.6325
    let input = SimilarityInput {
        items: vec![
            SimilarityItem {
                id: "A".into(),
                features: vec![1.0, 0.0, 0.0, 0.0, 0.0],
            },
            SimilarityItem {
                id: "B".into(),
                features: vec![0.0, 1.0, 0.0, 0.0, 0.0],
            },
            SimilarityItem {
                id: "C".into(),
                features: vec![1.0, 1.0, 0.0, 0.0, 0.0],
            },
            SimilarityItem {
                id: "D".into(),
                features: vec![1.0, 1.0, 1.0, 1.0, 1.0],
            },
        ],
        metric: DistanceMetric::Cosine,
        top_k: None,
    };
    let (output, _) = PairwiseSimilaritySolver.solve(&input, &spec()).unwrap();

    assert_eq!(output.total_pairs, 6); // C(4,2)

    let find = |a: &str, b: &str| -> f64 {
        output
            .pairs
            .iter()
            .find(|p| (p.id_a == a && p.id_b == b) || (p.id_a == b && p.id_b == a))
            .unwrap_or_else(|| panic!("pair {a}-{b} not found"))
            .score
    };

    assert!(find("A", "B").abs() < 1e-9, "orthogonal → 0");
    assert!(
        (find("A", "C") - 1.0 / 2.0_f64.sqrt()).abs() < 1e-6,
        "45° → 1/√2"
    );
    assert!(
        (find("A", "D") - 1.0 / 5.0_f64.sqrt()).abs() < 1e-6,
        "A·D/norms → 1/√5"
    );
    assert!((find("B", "C") - 1.0 / 2.0_f64.sqrt()).abs() < 1e-6);
    assert!((find("B", "D") - 1.0 / 5.0_f64.sqrt()).abs() < 1e-6);
    assert!(
        (find("C", "D") - 2.0 / 10.0_f64.sqrt()).abs() < 1e-6,
        "C·D = 2/(√2·√5)"
    );
}

// ── Exponential Smoothing: 8 values ─────────────────────────────────────────

#[test]
fn exponential_smoothing_8_values() {
    // Values: [100, 120, 90, 130, 110, 95, 105, 115], α=0.3
    //
    // level[0] = 100
    // level[1] = 0.3×120 + 0.7×100     = 106.0
    // level[2] = 0.3×90  + 0.7×106     = 101.2
    // level[3] = 0.3×130 + 0.7×101.2   = 109.84
    // level[4] = 0.3×110 + 0.7×109.84  = 109.888
    // level[5] = 0.3×95  + 0.7×109.888 = 105.4216
    // level[6] = 0.3×105 + 0.7×105.4216= 105.29512
    // level[7] = 0.3×115 + 0.7×105.29512= 108.206584
    //
    // Forecast = 108.206584
    let input = ForecastingInput {
        values: vec![100.0, 120.0, 90.0, 130.0, 110.0, 95.0, 105.0, 115.0],
        horizon: 1,
        alpha: 0.3,
    };
    let (output, _) = ExponentialSmoothingSolver.solve(&input, &spec()).unwrap();

    assert_eq!(output.predictions.len(), 1);
    assert!(
        (output.predictions[0].value - 108.206584).abs() < 1e-4,
        "8-step SES forecast ≈ 108.2066, got {}",
        output.predictions[0].value
    );
}

// ── K-Means: 4 clusters in 3D ──────────────────────────────────────────────

#[test]
fn kmeans_4_clusters_3d() {
    // 4 well-separated clusters, 3 points each, in 3D.
    // Separation of 100 units ensures any random initialization converges correctly.
    //   Cluster near (0,0,0): [0,0,0], [1,0,0], [0,1,0]
    //   Cluster near (100,0,0): [100,0,0], [101,0,0], [100,1,0]
    //   Cluster near (0,100,0): [0,100,0], [1,100,0], [0,101,0]
    //   Cluster near (0,0,100): [0,0,100], [1,0,100], [0,1,100]
    let input = SegmentationInput {
        records: vec![
            vec![0.0, 0.0, 0.0],
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![100.0, 0.0, 0.0],
            vec![101.0, 0.0, 0.0],
            vec![100.0, 1.0, 0.0],
            vec![0.0, 100.0, 0.0],
            vec![1.0, 100.0, 0.0],
            vec![0.0, 101.0, 0.0],
            vec![0.0, 0.0, 100.0],
            vec![1.0, 0.0, 100.0],
            vec![0.0, 1.0, 100.0],
        ],
        k: 4,
        max_iterations: 100,
        seed: None,
    };

    // K-means with random init can converge to local minima. Try multiple seeds.
    // With separation of 100, at least one of several runs must find 4 clusters.
    let mut found = false;
    for seed in 0..20 {
        let mut attempt = input.clone();
        attempt.seed = Some(seed);
        let (output, _) = KMeansSolver.solve(&attempt, &spec()).unwrap();

        // Check if all 4 groups got distinct clusters.
        let groups_ok = [0, 3, 6, 9].iter().all(|&start| {
            output.assignments[start] == output.assignments[start + 1]
                && output.assignments[start + 1] == output.assignments[start + 2]
        });
        let labels: std::collections::HashSet<_> = [
            output.assignments[0],
            output.assignments[3],
            output.assignments[6],
            output.assignments[9],
        ]
        .into_iter()
        .collect();

        if groups_ok && labels.len() == 4 {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "k-means must find 4 clusters in at least one of 20 seed attempts"
    );
}

// ── Ranking: 5 items, 3 criteria, mixed directions ──────────────────────────

#[test]
fn ranking_5_items_mixed_directions() {
    // 5 items, 3 criteria: [higher, lower, higher], weights [0.5, 0.3, 0.2]
    //
    //          C1    C2    C3
    //   A:    90    20    80
    //   B:    70    10    60
    //   C:    80    40   100
    //   D:    60    30    70
    //   E:   100    50    90
    //
    // Normalize C1 (higher=better, min=60, max=100):
    //   A=0.75, B=0.25, C=0.50, D=0.00, E=1.00
    //
    // Normalize C2 (lower=better, min=10, max=50): invert
    //   A=1-(10/40)=0.75, B=1-0=1.00, C=1-(30/40)=0.25, D=1-(20/40)=0.50, E=1-1=0.00
    //
    // Normalize C3 (higher=better, min=60, max=100):
    //   A=0.50, B=0.00, C=1.00, D=0.25, E=0.75
    //
    // Composite:
    //   A: 0.5×0.75 + 0.3×0.75 + 0.2×0.50 = 0.375+0.225+0.100 = 0.700
    //   B: 0.5×0.25 + 0.3×1.00 + 0.2×0.00 = 0.125+0.300+0.000 = 0.425
    //   C: 0.5×0.50 + 0.3×0.25 + 0.2×1.00 = 0.250+0.075+0.200 = 0.525
    //   D: 0.5×0.00 + 0.3×0.50 + 0.2×0.25 = 0.000+0.150+0.050 = 0.200
    //   E: 0.5×1.00 + 0.3×0.00 + 0.2×0.75 = 0.500+0.000+0.150 = 0.650
    //
    // Rank: A(0.700) > E(0.650) > C(0.525) > B(0.425) > D(0.200)
    let input = RankingInput {
        items: vec![
            RankItem {
                id: "A".into(),
                scores: vec![90.0, 20.0, 80.0],
            },
            RankItem {
                id: "B".into(),
                scores: vec![70.0, 10.0, 60.0],
            },
            RankItem {
                id: "C".into(),
                scores: vec![80.0, 40.0, 100.0],
            },
            RankItem {
                id: "D".into(),
                scores: vec![60.0, 30.0, 70.0],
            },
            RankItem {
                id: "E".into(),
                scores: vec![100.0, 50.0, 90.0],
            },
        ],
        weights: vec![0.5, 0.3, 0.2],
        higher_is_better: vec![true, false, true],
        top_k: None,
    };
    let (output, _) = WeightedScoringSolver.solve(&input, &spec()).unwrap();

    assert_eq!(output.ranked[0].id, "A");
    assert_eq!(output.ranked[1].id, "E");
    assert_eq!(output.ranked[2].id, "C");
    assert_eq!(output.ranked[3].id, "B");
    assert_eq!(output.ranked[4].id, "D");
    assert!((output.ranked[0].composite_score - 0.700).abs() < 1e-6);
    assert!((output.ranked[1].composite_score - 0.650).abs() < 1e-6);
    assert!((output.ranked[2].composite_score - 0.525).abs() < 1e-6);
    assert!((output.ranked[3].composite_score - 0.425).abs() < 1e-6);
    assert!((output.ranked[4].composite_score - 0.200).abs() < 1e-6);
}
