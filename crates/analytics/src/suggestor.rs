use converge_pack::ContextKey;
use converge_pack::PackSuggestor;

use crate::packs::{
    AnomalyDetectionPack, ClassificationPack, DescriptiveStatsPack, ForecastingPack, RankingPack,
    RegressionPack, SegmentationPack, SimilarityPack, TrendDetectionPack,
};

pub fn anomaly_detection() -> PackSuggestor<AnomalyDetectionPack> {
    PackSuggestor::new(
        AnomalyDetectionPack,
        ContextKey::Seeds,
        ContextKey::Strategies,
    )
}

pub fn classification() -> PackSuggestor<ClassificationPack> {
    PackSuggestor::new(
        ClassificationPack,
        ContextKey::Seeds,
        ContextKey::Strategies,
    )
}

pub fn descriptive_stats() -> PackSuggestor<DescriptiveStatsPack> {
    PackSuggestor::new(
        DescriptiveStatsPack,
        ContextKey::Seeds,
        ContextKey::Strategies,
    )
}

pub fn forecasting() -> PackSuggestor<ForecastingPack> {
    PackSuggestor::new(ForecastingPack, ContextKey::Seeds, ContextKey::Strategies)
}

pub fn ranking() -> PackSuggestor<RankingPack> {
    PackSuggestor::new(RankingPack, ContextKey::Seeds, ContextKey::Strategies)
}

pub fn regression() -> PackSuggestor<RegressionPack> {
    PackSuggestor::new(RegressionPack, ContextKey::Seeds, ContextKey::Strategies)
}

pub fn segmentation() -> PackSuggestor<SegmentationPack> {
    PackSuggestor::new(SegmentationPack, ContextKey::Seeds, ContextKey::Strategies)
}

pub fn similarity() -> PackSuggestor<SimilarityPack> {
    PackSuggestor::new(SimilarityPack, ContextKey::Seeds, ContextKey::Strategies)
}

pub fn trend_detection() -> PackSuggestor<TrendDetectionPack> {
    PackSuggestor::new(
        TrendDetectionPack,
        ContextKey::Seeds,
        ContextKey::Strategies,
    )
}
