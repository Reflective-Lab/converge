use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistanceMetric {
    Euclidean,
    Cosine,
    Manhattan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityInput {
    pub items: Vec<SimilarityItem>,
    #[serde(default = "default_metric")]
    pub metric: DistanceMetric,
    pub top_k: Option<usize>,
}

fn default_metric() -> DistanceMetric {
    DistanceMetric::Cosine
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityItem {
    pub id: String,
    pub features: Vec<f64>,
}

impl SimilarityInput {
    pub fn validate(&self) -> Result<()> {
        if self.items.len() < 2 {
            return Err(converge_pack::GateError::invalid_input(
                "At least 2 items required for similarity",
            ));
        }
        let dim = self.items[0].features.len();
        if dim == 0 {
            return Err(converge_pack::GateError::invalid_input(
                "Items must have at least one feature",
            ));
        }
        for (i, item) in self.items.iter().enumerate() {
            if item.features.len() != dim {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Item {} has {} features, expected {}",
                    i,
                    item.features.len(),
                    dim
                )));
            }
        }
        if let Some(top_k) = self.top_k {
            if top_k == 0 {
                return Err(converge_pack::GateError::invalid_input(
                    "top_k must be >= 1",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityPair {
    pub id_a: String,
    pub id_b: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityOutput {
    pub pairs: Vec<SimilarityPair>,
    pub total_pairs: usize,
}

impl SimilarityOutput {
    pub fn summary(&self) -> String {
        format!(
            "Computed {} similarity pairs (showing {})",
            self.total_pairs,
            self.pairs.len(),
        )
    }
}
