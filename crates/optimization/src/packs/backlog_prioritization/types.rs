//! Types for Backlog Prioritization pack

use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

/// Input for backlog prioritization optimization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BacklogPrioritizationInput {
    /// Backlog items to prioritize
    pub items: Vec<BacklogItem>,
    /// Total capacity in points
    pub capacity_points: i64,
}

impl BacklogPrioritizationInput {
    /// Validate the input
    pub fn validate(&self) -> Result<()> {
        if self.items.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one item is required",
            ));
        }
        if self.capacity_points <= 0 {
            return Err(converge_pack::GateError::invalid_input(
                "Capacity must be positive",
            ));
        }
        for item in &self.items {
            if item.effort_points <= 0 {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Item {} has invalid effort_points: {}",
                    item.id, item.effort_points
                )));
            }
        }
        Ok(())
    }

    /// Get item by ID
    pub fn get_item(&self, id: &str) -> Option<&BacklogItem> {
        self.items.iter().find(|i| i.id == id)
    }

    /// Build dependency graph
    pub fn build_dependency_order(&self) -> Vec<&BacklogItem> {
        // Simple topological sort
        let mut result = Vec::new();
        let mut remaining: Vec<_> = self.items.iter().collect();

        while !remaining.is_empty() {
            let before_len = remaining.len();

            remaining.retain(|item| {
                let deps_satisfied = item
                    .dependencies
                    .iter()
                    .all(|dep| result.iter().any(|i: &&BacklogItem| i.id == *dep));
                if deps_satisfied {
                    result.push(*item);
                    false
                } else {
                    true
                }
            });

            // If no progress, there's a cycle - just add remaining items
            if remaining.len() == before_len {
                result.extend(remaining.drain(..));
            }
        }

        result
    }
}

/// A backlog item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklogItem {
    /// Item identifier
    pub id: String,
    /// Item title
    pub title: String,
    /// Business value (0-100)
    pub business_value: f64,
    /// Time criticality (0-100, how urgent)
    pub time_criticality: f64,
    /// Risk reduction/opportunity enablement (0-100)
    pub risk_reduction: f64,
    /// Effort in story points
    pub effort_points: i64,
    /// Dependencies (item IDs that must be done first)
    pub dependencies: Vec<String>,
}

impl BacklogItem {
    /// Calculate WSJF score (Weighted Shortest Job First)
    /// WSJF = Cost of Delay / Job Size
    /// Cost of Delay = Business Value + Time Criticality + Risk Reduction
    pub fn wsjf_score(&self) -> f64 {
        let cost_of_delay = self.business_value + self.time_criticality + self.risk_reduction;
        cost_of_delay / self.effort_points as f64
    }

    /// Check if dependencies are satisfied
    pub fn dependencies_satisfied(&self, completed: &[&str]) -> bool {
        self.dependencies
            .iter()
            .all(|d| completed.contains(&d.as_str()))
    }
}

/// Output for backlog prioritization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BacklogPrioritizationOutput {
    /// Ranked items
    pub ranked_items: Vec<RankedItem>,
    /// Total value delivered
    pub total_value: f64,
    /// Total effort consumed
    pub total_effort: i64,
    /// Items included in capacity
    pub included_count: usize,
    /// Items excluded due to capacity
    pub excluded_count: usize,
}

impl BacklogPrioritizationOutput {
    /// Create empty output
    pub fn empty(_reason: &str) -> Self {
        Self {
            ranked_items: vec![],
            total_value: 0.0,
            total_effort: 0,
            included_count: 0,
            excluded_count: 0,
        }
    }

    /// Generate a summary string
    pub fn summary(&self) -> String {
        format!(
            "Prioritized {} items ({} included in capacity), total value: {:.1}",
            self.ranked_items.len(),
            self.included_count,
            self.total_value
        )
    }
}

/// A ranked backlog item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedItem {
    /// Item identifier
    pub item_id: String,
    /// Item title
    pub item_title: String,
    /// Priority rank (1 = highest)
    pub rank: usize,
    /// WSJF score
    pub wsjf_score: f64,
    /// Whether item fits in capacity
    pub included_in_capacity: bool,
    /// Cumulative effort at this point
    pub cumulative_effort: i64,
    /// Why this item was ranked here
    pub ranking_reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_item(id: &str, value: f64, effort: i64) -> BacklogItem {
        BacklogItem {
            id: id.to_string(),
            title: format!("Item {}", id),
            business_value: value,
            time_criticality: 50.0,
            risk_reduction: 30.0,
            effort_points: effort,
            dependencies: vec![],
        }
    }

    #[test]
    fn test_wsjf_calculation() {
        let item = create_test_item("i1", 80.0, 5);
        // (80 + 50 + 30) / 5 = 32
        assert!((item.wsjf_score() - 32.0).abs() < 0.01);
    }

    #[test]
    fn test_dependencies_satisfied() {
        let mut item = create_test_item("i2", 50.0, 3);
        item.dependencies = vec!["i1".to_string()];

        assert!(!item.dependencies_satisfied(&[]));
        assert!(item.dependencies_satisfied(&["i1"]));
    }

    #[test]
    fn test_dependency_order() {
        let input = BacklogPrioritizationInput {
            items: vec![
                BacklogItem {
                    id: "i1".to_string(),
                    title: "First".to_string(),
                    business_value: 50.0,
                    time_criticality: 50.0,
                    risk_reduction: 50.0,
                    effort_points: 3,
                    dependencies: vec![],
                },
                BacklogItem {
                    id: "i2".to_string(),
                    title: "Second".to_string(),
                    business_value: 80.0,
                    time_criticality: 50.0,
                    risk_reduction: 50.0,
                    effort_points: 5,
                    dependencies: vec!["i1".to_string()],
                },
            ],
            capacity_points: 20,
        };

        let order = input.build_dependency_order();
        assert_eq!(order[0].id, "i1");
        assert_eq!(order[1].id, "i2");
    }

    #[test]
    fn test_input_validation() {
        let mut input = BacklogPrioritizationInput {
            items: vec![create_test_item("i1", 50.0, 3)],
            capacity_points: 10,
        };

        assert!(input.validate().is_ok());

        input.capacity_points = 0;
        assert!(input.validate().is_err());
    }
}
