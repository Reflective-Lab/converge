//! Types for Budget Allocation pack

use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

/// Input for budget allocation optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAllocationInput {
    /// Total budget to allocate
    pub total_budget: f64,
    /// Categories to allocate budget to
    pub categories: Vec<BudgetCategory>,
    /// Allocation constraints
    pub constraints: AllocationConstraints,
}

impl BudgetAllocationInput {
    /// Validate the input
    pub fn validate(&self) -> Result<()> {
        if !self.total_budget.is_finite() || self.total_budget <= 0.0 {
            return Err(converge_pack::GateError::invalid_input(
                "Total budget must be finite and positive",
            ));
        }
        if self.categories.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one category is required",
            ));
        }
        if !self.constraints.min_roi_threshold.is_finite()
            || !(0.0..=1.0).contains(&self.constraints.min_roi_threshold)
        {
            return Err(converge_pack::GateError::invalid_input(
                "Minimum ROI threshold must be finite and between 0 and 1",
            ));
        }
        if matches!(self.constraints.max_categories, Some(0)) {
            return Err(converge_pack::GateError::invalid_input(
                "max_categories must be positive when set",
            ));
        }

        for category in &self.categories {
            if !category.expected_roi.is_finite() || !(0.0..=1.0).contains(&category.expected_roi) {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Category {} has invalid expected_roi: must be finite and between 0 and 1",
                    category.id
                )));
            }
            if !category.priority_weight.is_finite() || category.priority_weight < 0.0 {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Category {} has invalid priority_weight: must be finite and non-negative",
                    category.id
                )));
            }
            if !category.min_allocation.is_finite() || category.min_allocation < 0.0 {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Category {} has invalid min_allocation: must be finite and non-negative",
                    category.id
                )));
            }
            if !category.max_allocation.is_finite() || category.max_allocation < 0.0 {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Category {} has invalid max_allocation: must be finite and non-negative",
                    category.id
                )));
            }
            if category.max_allocation < category.min_allocation {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Category {} has max_allocation below min_allocation",
                    category.id
                )));
            }
        }

        // Check minimum allocations don't exceed budget
        let total_min: f64 = self.categories.iter().map(|c| c.min_allocation).sum();
        if !total_min.is_finite() || total_min > self.total_budget {
            return Err(converge_pack::GateError::invalid_input(format!(
                "Minimum allocations ({:.2}) exceed total budget ({:.2})",
                total_min, self.total_budget
            )));
        }

        Ok(())
    }

    /// Get category by ID
    pub fn get_category(&self, id: &str) -> Option<&BudgetCategory> {
        self.categories.iter().find(|c| c.id == id)
    }
}

/// A budget category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetCategory {
    /// Category identifier
    pub id: String,
    /// Category name
    pub name: String,
    /// Expected ROI (0-1 scale, or percentage)
    pub expected_roi: f64,
    /// Priority weight (higher = more important)
    pub priority_weight: f64,
    /// Minimum required allocation
    pub min_allocation: f64,
    /// Maximum allowed allocation
    pub max_allocation: f64,
}

impl BudgetCategory {
    /// Calculate efficiency score (ROI * priority)
    pub fn efficiency_score(&self) -> f64 {
        self.expected_roi * self.priority_weight
    }

    /// Check if an allocation is valid for this category
    pub fn is_valid_allocation(&self, amount: f64) -> bool {
        amount >= self.min_allocation && amount <= self.max_allocation
    }
}

/// Allocation constraints
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AllocationConstraints {
    /// Maximum number of categories to fund
    pub max_categories: Option<usize>,
    /// Minimum ROI threshold to fund
    pub min_roi_threshold: f64,
    /// Whether to allow partial funding (below min but above 0)
    pub allow_partial: bool,
}

/// Output for budget allocation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BudgetAllocationOutput {
    /// Allocations per category
    pub allocations: Vec<CategoryAllocation>,
    /// Total budget allocated
    pub total_allocated: f64,
    /// Total expected return
    pub total_expected_return: f64,
    /// Budget remaining (unallocated)
    pub budget_remaining: f64,
    /// Overall portfolio ROI
    pub portfolio_roi: f64,
}

impl BudgetAllocationOutput {
    /// Create empty output
    pub fn empty(budget: f64) -> Self {
        Self {
            allocations: vec![],
            total_allocated: 0.0,
            total_expected_return: 0.0,
            budget_remaining: budget,
            portfolio_roi: 0.0,
        }
    }

    /// Generate a summary string
    pub fn summary(&self) -> String {
        format!(
            "Allocated ${:.2} across {} categories, expected ROI: {:.1}%",
            self.total_allocated,
            self.allocations.iter().filter(|a| a.amount > 0.0).count(),
            self.portfolio_roi * 100.0
        )
    }
}

/// Allocation for a single category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryAllocation {
    /// Category identifier
    pub category_id: String,
    /// Category name
    pub category_name: String,
    /// Allocated amount
    pub amount: f64,
    /// Percentage of total budget
    pub percentage: f64,
    /// Expected return from this allocation
    pub expected_return: f64,
    /// Reason for allocation decision
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_category(id: &str, roi: f64, priority: f64) -> BudgetCategory {
        BudgetCategory {
            id: id.to_string(),
            name: format!("Category {}", id),
            expected_roi: roi,
            priority_weight: priority,
            min_allocation: 1000.0,
            max_allocation: 50000.0,
        }
    }

    #[test]
    fn test_efficiency_score() {
        let category = create_test_category("c1", 0.15, 2.0);
        assert!((category.efficiency_score() - 0.30).abs() < 0.01);
    }

    #[test]
    fn test_valid_allocation() {
        let category = create_test_category("c1", 0.15, 2.0);
        assert!(category.is_valid_allocation(10000.0));
        assert!(!category.is_valid_allocation(500.0)); // Below min
        assert!(!category.is_valid_allocation(60000.0)); // Above max
    }

    #[test]
    fn test_input_validation() {
        let mut input = BudgetAllocationInput {
            total_budget: 100000.0,
            categories: vec![create_test_category("c1", 0.15, 2.0)],
            constraints: AllocationConstraints::default(),
        };

        assert!(input.validate().is_ok());

        input.total_budget = 0.0;
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_min_exceeds_budget() {
        let input = BudgetAllocationInput {
            total_budget: 500.0, // Less than min_allocation of 1000
            categories: vec![create_test_category("c1", 0.15, 2.0)],
            constraints: AllocationConstraints::default(),
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_rejects_non_finite_or_out_of_range_values() {
        let mut input = BudgetAllocationInput {
            total_budget: f64::NAN,
            categories: vec![create_test_category("c1", 0.15, 2.0)],
            constraints: AllocationConstraints::default(),
        };
        assert!(input.validate().is_err());

        input.total_budget = 100000.0;
        input.categories[0].expected_roi = 1.1;
        assert!(input.validate().is_err());

        input.categories[0].expected_roi = 0.15;
        input.categories[0].priority_weight = f64::INFINITY;
        assert!(input.validate().is_err());

        input.categories[0].priority_weight = 2.0;
        input.categories[0].min_allocation = 60000.0;
        input.categories[0].max_allocation = 50000.0;
        assert!(input.validate().is_err());

        input.categories[0].min_allocation = 1000.0;
        input.categories[0].max_allocation = 50000.0;
        input.constraints.min_roi_threshold = -0.1;
        assert!(input.validate().is_err());
    }
}
