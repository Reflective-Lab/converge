//! Types for Capacity Planning pack

use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

/// Input for capacity planning optimization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapacityPlanningInput {
    /// Demand forecasts by period
    pub demand_forecasts: Vec<DemandForecast>,
    /// Available resource types
    pub resource_types: Vec<ResourceType>,
    /// Teams with their capacity
    pub teams: Vec<Team>,
    /// Planning constraints
    pub constraints: PlanningConstraints,
}

impl CapacityPlanningInput {
    /// Validate the input
    pub fn validate(&self) -> Result<()> {
        if self.demand_forecasts.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one demand forecast is required",
            ));
        }
        if self.resource_types.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one resource type is required",
            ));
        }
        if self.teams.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one team is required",
            ));
        }

        // Validate demand quantities are non-negative
        for forecast in &self.demand_forecasts {
            if forecast.demand_units < 0.0 {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Demand units cannot be negative for period {}",
                    forecast.period_id
                )));
            }
        }

        // Validate team capacities are non-negative
        for team in &self.teams {
            if team.available_capacity < 0.0 {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Available capacity cannot be negative for team {}",
                    team.id
                )));
            }
        }

        Ok(())
    }

    /// Get total demand across all periods
    pub fn total_demand(&self) -> f64 {
        self.demand_forecasts.iter().map(|f| f.demand_units).sum()
    }

    /// Get total capacity across all teams
    pub fn total_capacity(&self) -> f64 {
        self.teams.iter().map(|t| t.available_capacity).sum()
    }

    /// Get teams with a specific skill
    pub fn teams_with_skill<'a>(&'a self, skill: &'a str) -> impl Iterator<Item = &'a Team> + 'a {
        self.teams
            .iter()
            .filter(move |t| t.skills.iter().any(|s| s == skill))
    }

    /// Get demand for a specific period
    pub fn demand_for_period(&self, period_id: &str) -> Option<&DemandForecast> {
        self.demand_forecasts
            .iter()
            .find(|f| f.period_id == period_id)
    }
}

/// Demand forecast for a planning period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemandForecast {
    /// Period identifier (e.g., "2024-Q1", "week-12")
    pub period_id: String,
    /// Resource type needed
    pub resource_type: String,
    /// Required skill for this demand
    pub required_skill: String,
    /// Forecasted demand in units
    pub demand_units: f64,
    /// Priority level (1 = highest)
    pub priority: u32,
    /// Minimum fulfillment ratio required (0.0 - 1.0)
    pub min_fulfillment_ratio: f64,
}

impl DemandForecast {
    /// Check if demand is fully covered by a given allocation
    pub fn is_fully_met(&self, allocated: f64) -> bool {
        allocated >= self.demand_units
    }

    /// Calculate fulfillment ratio for a given allocation
    pub fn fulfillment_ratio(&self, allocated: f64) -> f64 {
        if self.demand_units == 0.0 {
            1.0
        } else {
            (allocated / self.demand_units).min(1.0)
        }
    }

    /// Check if minimum fulfillment is achieved
    pub fn meets_minimum(&self, allocated: f64) -> bool {
        self.fulfillment_ratio(allocated) >= self.min_fulfillment_ratio
    }
}

/// A type of resource that can be allocated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceType {
    /// Resource type identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Unit of measurement (e.g., "hours", "FTE", "units")
    pub unit: String,
    /// Cost per unit
    pub cost_per_unit: f64,
}

impl ResourceType {
    /// Calculate cost for a given quantity
    pub fn cost_for(&self, units: f64) -> f64 {
        units * self.cost_per_unit
    }
}

/// A team that can fulfill demand
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    /// Team identifier
    pub id: String,
    /// Team name
    pub name: String,
    /// Skills this team possesses
    pub skills: Vec<String>,
    /// Resource types this team can provide
    pub resource_types: Vec<String>,
    /// Available capacity in units
    pub available_capacity: f64,
    /// Maximum utilization allowed (0.0 - 1.0)
    pub max_utilization: f64,
    /// Current headcount
    pub headcount: i32,
}

impl Team {
    /// Check if team has a specific skill
    pub fn has_skill(&self, skill: &str) -> bool {
        self.skills.iter().any(|s| s == skill)
    }

    /// Check if team can provide a specific resource type
    pub fn provides_resource_type(&self, resource_type: &str) -> bool {
        self.resource_types.iter().any(|r| r == resource_type)
    }

    /// Get effective capacity after applying max utilization
    pub fn effective_capacity(&self) -> f64 {
        self.available_capacity * self.max_utilization
    }

    /// Calculate utilization for a given allocation
    pub fn utilization(&self, allocated: f64) -> f64 {
        if self.available_capacity == 0.0 {
            0.0
        } else {
            allocated / self.available_capacity
        }
    }

    /// Check if allocation would exceed max utilization
    pub fn would_exceed_utilization(&self, allocated: f64) -> bool {
        self.utilization(allocated) > self.max_utilization
    }
}

/// Planning constraints
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanningConstraints {
    /// Target utilization for teams (0.0 - 1.0)
    pub target_utilization: f64,
    /// Maximum total budget
    pub max_budget: Option<f64>,
    /// Minimum fulfillment ratio across all demands
    pub min_overall_fulfillment: f64,
    /// Whether cross-team allocation is allowed
    pub allow_cross_team: bool,
    /// Skills that must be matched exactly (vs. flexible matching)
    pub strict_skill_matching: bool,
}

/// Output for capacity planning optimization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapacityPlanningOutput {
    /// Resource assignments
    pub assignments: Vec<ResourceAssignment>,
    /// Per-team utilization metrics
    pub team_utilization: Vec<TeamUtilization>,
    /// Per-period fulfillment metrics
    pub period_fulfillment: Vec<PeriodFulfillment>,
    /// Summary statistics
    pub summary: CapacityPlanSummary,
}

impl CapacityPlanningOutput {
    /// Create output when no feasible plan exists
    pub fn infeasible(reason: &str) -> Self {
        Self {
            assignments: vec![],
            team_utilization: vec![],
            period_fulfillment: vec![],
            summary: CapacityPlanSummary {
                total_demand: 0.0,
                total_allocated: 0.0,
                overall_fulfillment_ratio: 0.0,
                total_cost: 0.0,
                average_utilization: 0.0,
                teams_over_capacity: 0,
                unmet_demands: 0,
                plan_status: format!("Infeasible: {}", reason),
            },
        }
    }

    /// Generate a summary string
    pub fn summary(&self) -> String {
        format!(
            "Allocated {:.1} of {:.1} units ({:.0}% fulfillment), avg utilization {:.0}%",
            self.summary.total_allocated,
            self.summary.total_demand,
            self.summary.overall_fulfillment_ratio * 100.0,
            self.summary.average_utilization * 100.0
        )
    }

    /// Check if all demands are met
    pub fn all_demands_met(&self) -> bool {
        self.summary.unmet_demands == 0 && self.summary.overall_fulfillment_ratio >= 0.999
    }

    /// Check if any team is over capacity
    pub fn has_overallocation(&self) -> bool {
        self.summary.teams_over_capacity > 0
    }
}

/// A resource assignment from a team to a demand
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAssignment {
    /// Assignment identifier
    pub id: String,
    /// Team providing the resource
    pub team_id: String,
    /// Period for the assignment
    pub period_id: String,
    /// Resource type being allocated
    pub resource_type: String,
    /// Demand being fulfilled
    pub demand_id: String,
    /// Units allocated
    pub allocated_units: f64,
    /// Cost of this assignment
    pub cost: f64,
}

/// Utilization metrics for a team
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamUtilization {
    /// Team identifier
    pub team_id: String,
    /// Team name
    pub team_name: String,
    /// Total capacity available
    pub total_capacity: f64,
    /// Total units allocated
    pub allocated: f64,
    /// Utilization ratio (allocated / capacity)
    pub utilization_ratio: f64,
    /// Remaining capacity
    pub remaining_capacity: f64,
    /// Whether team is over their max utilization
    pub is_over_utilized: bool,
}

/// Fulfillment metrics for a period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodFulfillment {
    /// Period identifier
    pub period_id: String,
    /// Total demand for this period
    pub total_demand: f64,
    /// Total allocated for this period
    pub total_allocated: f64,
    /// Fulfillment ratio
    pub fulfillment_ratio: f64,
    /// Unmet demand details
    pub unmet_demands: Vec<UnmetDemand>,
}

/// Details about unmet demand
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnmetDemand {
    /// Demand ID
    pub demand_id: String,
    /// Required skill
    pub required_skill: String,
    /// Units requested
    pub requested: f64,
    /// Units allocated
    pub allocated: f64,
    /// Shortfall
    pub shortfall: f64,
    /// Reason for shortfall
    pub reason: String,
}

/// Summary statistics for the capacity plan
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapacityPlanSummary {
    /// Total demand across all periods
    pub total_demand: f64,
    /// Total units allocated
    pub total_allocated: f64,
    /// Overall fulfillment ratio
    pub overall_fulfillment_ratio: f64,
    /// Total cost of allocations
    pub total_cost: f64,
    /// Average team utilization
    pub average_utilization: f64,
    /// Number of teams exceeding max utilization
    pub teams_over_capacity: usize,
    /// Number of demands not fully met
    pub unmet_demands: usize,
    /// Overall plan status
    pub plan_status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_input() -> CapacityPlanningInput {
        CapacityPlanningInput {
            demand_forecasts: vec![
                DemandForecast {
                    period_id: "Q1-2024".to_string(),
                    resource_type: "engineering".to_string(),
                    required_skill: "backend".to_string(),
                    demand_units: 100.0,
                    priority: 1,
                    min_fulfillment_ratio: 0.8,
                },
                DemandForecast {
                    period_id: "Q1-2024".to_string(),
                    resource_type: "engineering".to_string(),
                    required_skill: "frontend".to_string(),
                    demand_units: 50.0,
                    priority: 2,
                    min_fulfillment_ratio: 0.7,
                },
            ],
            resource_types: vec![ResourceType {
                id: "engineering".to_string(),
                name: "Engineering Hours".to_string(),
                unit: "hours".to_string(),
                cost_per_unit: 100.0,
            }],
            teams: vec![
                Team {
                    id: "team-a".to_string(),
                    name: "Backend Team".to_string(),
                    skills: vec!["backend".to_string()],
                    resource_types: vec!["engineering".to_string()],
                    available_capacity: 120.0,
                    max_utilization: 0.85,
                    headcount: 6,
                },
                Team {
                    id: "team-b".to_string(),
                    name: "Frontend Team".to_string(),
                    skills: vec!["frontend".to_string()],
                    resource_types: vec!["engineering".to_string()],
                    available_capacity: 80.0,
                    max_utilization: 0.85,
                    headcount: 4,
                },
            ],
            constraints: PlanningConstraints {
                target_utilization: 0.75,
                max_budget: Some(20000.0),
                min_overall_fulfillment: 0.8,
                allow_cross_team: false,
                strict_skill_matching: true,
            },
        }
    }

    #[test]
    fn test_input_validation() {
        let input = create_test_input();
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_input_validation_empty_demands() {
        let mut input = create_test_input();
        input.demand_forecasts = vec![];
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_input_validation_negative_demand() {
        let mut input = create_test_input();
        input.demand_forecasts[0].demand_units = -10.0;
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_total_demand_and_capacity() {
        let input = create_test_input();
        assert!((input.total_demand() - 150.0).abs() < 0.01);
        assert!((input.total_capacity() - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_demand_fulfillment_ratio() {
        let forecast = DemandForecast {
            period_id: "Q1".to_string(),
            resource_type: "eng".to_string(),
            required_skill: "backend".to_string(),
            demand_units: 100.0,
            priority: 1,
            min_fulfillment_ratio: 0.8,
        };

        assert!((forecast.fulfillment_ratio(50.0) - 0.5).abs() < 0.01);
        assert!((forecast.fulfillment_ratio(100.0) - 1.0).abs() < 0.01);
        assert!((forecast.fulfillment_ratio(150.0) - 1.0).abs() < 0.01); // Capped at 1.0
        assert!(!forecast.meets_minimum(50.0));
        assert!(forecast.meets_minimum(80.0));
    }

    #[test]
    fn test_team_utilization() {
        let team = Team {
            id: "t1".to_string(),
            name: "Team 1".to_string(),
            skills: vec!["backend".to_string()],
            resource_types: vec!["engineering".to_string()],
            available_capacity: 100.0,
            max_utilization: 0.85,
            headcount: 5,
        };

        assert!((team.effective_capacity() - 85.0).abs() < 0.01);
        assert!((team.utilization(80.0) - 0.8).abs() < 0.01);
        assert!(!team.would_exceed_utilization(80.0));
        assert!(team.would_exceed_utilization(90.0));
    }

    #[test]
    fn test_team_skills() {
        let team = Team {
            id: "t1".to_string(),
            name: "Team 1".to_string(),
            skills: vec!["backend".to_string(), "devops".to_string()],
            resource_types: vec!["engineering".to_string()],
            available_capacity: 100.0,
            max_utilization: 0.85,
            headcount: 5,
        };

        assert!(team.has_skill("backend"));
        assert!(team.has_skill("devops"));
        assert!(!team.has_skill("frontend"));
    }

    #[test]
    fn test_resource_type_cost() {
        let resource_type = ResourceType {
            id: "eng".to_string(),
            name: "Engineering".to_string(),
            unit: "hours".to_string(),
            cost_per_unit: 150.0,
        };

        assert!((resource_type.cost_for(10.0) - 1500.0).abs() < 0.01);
    }
}
