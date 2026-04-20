use converge_optimization::SolverSuggestor;
use converge_pack::ContextKey;

use crate::packs::{
    AnomalyDetectionPack, ClassificationPack, DescriptiveStatsPack, ForecastingPack, RankingPack,
    RegressionPack, SegmentationPack, SimilarityPack, TrendDetectionPack,
};

pub fn anomaly_detection() -> SolverSuggestor<AnomalyDetectionPack> {
    SolverSuggestor::new(
        AnomalyDetectionPack,
        ContextKey::Seeds,
        ContextKey::Strategies,
    )
}

pub fn classification() -> SolverSuggestor<ClassificationPack> {
    SolverSuggestor::new(
        ClassificationPack,
        ContextKey::Seeds,
        ContextKey::Strategies,
    )
}

pub fn descriptive_stats() -> SolverSuggestor<DescriptiveStatsPack> {
    SolverSuggestor::new(
        DescriptiveStatsPack,
        ContextKey::Seeds,
        ContextKey::Strategies,
    )
}

pub fn forecasting() -> SolverSuggestor<ForecastingPack> {
    SolverSuggestor::new(ForecastingPack, ContextKey::Seeds, ContextKey::Strategies)
}

pub fn ranking() -> SolverSuggestor<RankingPack> {
    SolverSuggestor::new(RankingPack, ContextKey::Seeds, ContextKey::Strategies)
}

pub fn regression() -> SolverSuggestor<RegressionPack> {
    SolverSuggestor::new(RegressionPack, ContextKey::Seeds, ContextKey::Strategies)
}

pub fn segmentation() -> SolverSuggestor<SegmentationPack> {
    SolverSuggestor::new(SegmentationPack, ContextKey::Seeds, ContextKey::Strategies)
}

pub fn similarity() -> SolverSuggestor<SimilarityPack> {
    SolverSuggestor::new(SimilarityPack, ContextKey::Seeds, ContextKey::Strategies)
}

pub fn trend_detection() -> SolverSuggestor<TrendDetectionPack> {
    SolverSuggestor::new(
        TrendDetectionPack,
        ContextKey::Seeds,
        ContextKey::Strategies,
    )
}
