use converge_optimization::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankItem {
    pub id: String,
    pub scores: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingInput {
    pub items: Vec<RankItem>,
    pub weights: Vec<f64>,
    pub higher_is_better: Vec<bool>,
    pub top_k: Option<usize>,
}

impl RankingInput {
    pub fn validate(&self) -> Result<()> {
        if self.items.is_empty() {
            return Err(converge_optimization::Error::invalid_input(
                "At least one item required",
            ));
        }
        let dim = self.weights.len();
        if dim == 0 {
            return Err(converge_optimization::Error::invalid_input(
                "At least one criterion required",
            ));
        }
        if self.higher_is_better.len() != dim {
            return Err(converge_optimization::Error::invalid_input(format!(
                "higher_is_better length {} must match weights length {}",
                self.higher_is_better.len(),
                dim
            )));
        }
        for weight in &self.weights {
            if *weight < 0.0 {
                return Err(converge_optimization::Error::invalid_input(
                    "All weights must be >= 0",
                ));
            }
        }
        for (i, item) in self.items.iter().enumerate() {
            if item.scores.len() != dim {
                return Err(converge_optimization::Error::invalid_input(format!(
                    "Item {} has {} scores, expected {}",
                    i,
                    item.scores.len(),
                    dim
                )));
            }
        }
        if let Some(top_k) = self.top_k {
            if top_k == 0 {
                return Err(converge_optimization::Error::invalid_input(
                    "top_k must be >= 1",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedItem {
    pub id: String,
    pub rank: usize,
    pub composite_score: f64,
    pub normalized_scores: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingOutput {
    pub ranked: Vec<RankedItem>,
    pub total_items: usize,
}

impl RankingOutput {
    pub fn summary(&self) -> String {
        format!(
            "Ranked {} items, top: {}",
            self.total_items,
            self.ranked.first().map(|r| r.id.as_str()).unwrap_or("none")
        )
    }
}
