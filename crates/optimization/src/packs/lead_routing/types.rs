//! Types for Lead Routing pack

use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Input for lead routing optimization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LeadRoutingInput {
    /// Leads to route
    pub leads: Vec<Lead>,
    /// Available sales reps
    pub reps: Vec<SalesRep>,
    /// Routing configuration
    pub config: RoutingConfig,
}

impl LeadRoutingInput {
    /// Validate the input
    pub fn validate(&self) -> Result<()> {
        if self.leads.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one lead is required",
            ));
        }
        if self.reps.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one sales rep is required",
            ));
        }

        // Check for duplicate lead IDs
        let mut seen_leads = HashSet::new();
        for lead in &self.leads {
            if !seen_leads.insert(&lead.id) {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Duplicate lead ID: {}",
                    lead.id
                )));
            }
        }

        // Check for duplicate rep IDs
        let mut seen_reps = HashSet::new();
        for rep in &self.reps {
            if !seen_reps.insert(&rep.id) {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Duplicate rep ID: {}",
                    rep.id
                )));
            }
            if rep.capacity < rep.current_load {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Rep {} has current_load ({}) exceeding capacity ({})",
                    rep.id, rep.current_load, rep.capacity
                )));
            }
        }

        Ok(())
    }

    /// Get total available capacity across all reps
    pub fn total_available_capacity(&self) -> i64 {
        self.reps.iter().map(|r| r.available_capacity()).sum()
    }

    /// Get reps with available capacity
    pub fn reps_with_capacity(&self) -> impl Iterator<Item = &SalesRep> {
        self.reps.iter().filter(|r| r.available_capacity() > 0)
    }

    /// Get reps that can handle a specific territory
    pub fn reps_for_territory(&self, territory: &str) -> impl Iterator<Item = &SalesRep> {
        let territory = territory.to_string();
        self.reps
            .iter()
            .filter(move |r| r.covers_territory(&territory))
    }
}

/// A lead to route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lead {
    /// Lead identifier
    pub id: String,
    /// Lead score (0-100, higher is better)
    pub score: f64,
    /// Territory/region the lead is in
    pub territory: String,
    /// Industry segment
    pub segment: String,
    /// Required expertise/skills for this lead
    #[serde(default)]
    pub required_skills: Vec<String>,
    /// Estimated deal value
    #[serde(default)]
    pub estimated_value: f64,
    /// Priority level (1 = highest)
    #[serde(default = "default_priority")]
    pub priority: i32,
}

fn default_priority() -> i32 {
    5
}

impl Lead {
    /// Check if a rep can handle this lead based on skills
    pub fn rep_has_required_skills(&self, rep: &SalesRep) -> bool {
        self.required_skills
            .iter()
            .all(|skill| rep.skills.contains(skill))
    }

    /// Calculate fit score with a rep (higher is better)
    pub fn calculate_fit_score(&self, rep: &SalesRep) -> f64 {
        let mut fit = 0.0;

        // Territory match: +40 points
        if rep.covers_territory(&self.territory) {
            fit += 40.0;
        }

        // Segment match: +30 points
        if rep.segments.contains(&self.segment) {
            fit += 30.0;
        }

        // Skills match: up to +20 points
        if !self.required_skills.is_empty() {
            let matched = self
                .required_skills
                .iter()
                .filter(|s| rep.skills.contains(*s))
                .count();
            let skill_ratio = matched as f64 / self.required_skills.len() as f64;
            fit += skill_ratio * 20.0;
        } else {
            fit += 20.0; // Full points if no skills required
        }

        // Rep performance bonus: up to +10 points
        fit += (rep.performance_score / 100.0) * 10.0;

        fit
    }
}

/// A sales rep
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesRep {
    /// Rep identifier
    pub id: String,
    /// Rep name
    pub name: String,
    /// Maximum leads this rep can handle
    pub capacity: i64,
    /// Current number of active leads
    pub current_load: i64,
    /// Territories this rep covers
    pub territories: Vec<String>,
    /// Segments this rep specializes in
    pub segments: Vec<String>,
    /// Skills/expertise areas
    #[serde(default)]
    pub skills: Vec<String>,
    /// Performance score (0-100)
    #[serde(default = "default_performance")]
    pub performance_score: f64,
}

fn default_performance() -> f64 {
    50.0
}

impl SalesRep {
    /// Get available capacity
    pub fn available_capacity(&self) -> i64 {
        (self.capacity - self.current_load).max(0)
    }

    /// Check if rep covers a territory
    pub fn covers_territory(&self, territory: &str) -> bool {
        self.territories.iter().any(|t| t == territory || t == "*")
    }

    /// Check if rep can take on more leads
    pub fn can_accept_lead(&self) -> bool {
        self.available_capacity() > 0
    }

    /// Calculate suitability for a lead (considering capacity)
    pub fn suitability_for_lead(&self, lead: &Lead) -> f64 {
        if !self.can_accept_lead() {
            return 0.0;
        }

        let fit_score = lead.calculate_fit_score(self);

        // Apply capacity penalty as load increases
        let load_ratio = self.current_load as f64 / self.capacity as f64;
        let capacity_factor = 1.0 - (load_ratio * 0.3); // Max 30% penalty at full capacity

        fit_score * capacity_factor
    }
}

/// Routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Whether territory matching is required (vs preferred)
    #[serde(default)]
    pub require_territory_match: bool,
    /// Whether to balance load across reps
    #[serde(default = "default_true")]
    pub balance_load: bool,
    /// Weight for territory fit in scoring (0-1)
    #[serde(default = "default_territory_weight")]
    pub territory_weight: f64,
    /// Weight for expertise fit in scoring (0-1)
    #[serde(default = "default_expertise_weight")]
    pub expertise_weight: f64,
    /// Weight for capacity/load balancing (0-1)
    #[serde(default = "default_capacity_weight")]
    pub capacity_weight: f64,
}

fn default_true() -> bool {
    true
}

fn default_territory_weight() -> f64 {
    0.4
}

fn default_expertise_weight() -> f64 {
    0.35
}

fn default_capacity_weight() -> f64 {
    0.25
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            require_territory_match: false,
            balance_load: true,
            territory_weight: default_territory_weight(),
            expertise_weight: default_expertise_weight(),
            capacity_weight: default_capacity_weight(),
        }
    }
}

/// Output for lead routing optimization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LeadRoutingOutput {
    /// Lead assignments
    pub assignments: Vec<LeadAssignment>,
    /// Leads that could not be assigned
    pub unassigned: Vec<UnassignedLead>,
    /// Rep utilization stats
    pub rep_utilization: Vec<RepUtilization>,
    /// Summary statistics
    pub stats: RoutingStats,
}

impl LeadRoutingOutput {
    /// Create output when no assignments possible
    pub fn no_assignments(reason: &str, leads: &[Lead]) -> Self {
        Self {
            assignments: vec![],
            unassigned: leads
                .iter()
                .map(|l| UnassignedLead {
                    lead_id: l.id.clone(),
                    reason: reason.to_string(),
                })
                .collect(),
            rep_utilization: vec![],
            stats: RoutingStats {
                total_leads: leads.len(),
                assigned_leads: 0,
                unassigned_leads: leads.len(),
                average_fit_score: 0.0,
                total_estimated_value: 0.0,
                summary: reason.to_string(),
            },
        }
    }

    /// Generate a summary string
    pub fn summary(&self) -> String {
        format!(
            "Assigned {} of {} leads to {} reps (avg fit: {:.1})",
            self.stats.assigned_leads,
            self.stats.total_leads,
            self.rep_utilization.len(),
            self.stats.average_fit_score
        )
    }
}

/// A lead assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeadAssignment {
    /// Lead identifier
    pub lead_id: String,
    /// Assigned rep identifier
    pub rep_id: String,
    /// Assigned rep name
    pub rep_name: String,
    /// Fit score for this assignment (0-100)
    pub fit_score: f64,
    /// Breakdown of scoring factors
    pub scoring_rationale: ScoringRationale,
}

/// Scoring rationale breakdown
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoringRationale {
    /// Points from territory match
    pub territory_score: f64,
    /// Points from segment match
    pub segment_score: f64,
    /// Points from skills match
    pub skills_score: f64,
    /// Points from rep performance
    pub performance_score: f64,
    /// Capacity adjustment factor
    pub capacity_factor: f64,
    /// Brief explanation
    pub explanation: String,
}

/// An unassigned lead with reason
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnassignedLead {
    /// Lead identifier
    pub lead_id: String,
    /// Reason for not assigning
    pub reason: String,
}

/// Rep utilization statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepUtilization {
    /// Rep identifier
    pub rep_id: String,
    /// Rep name
    pub rep_name: String,
    /// Number of leads assigned in this routing
    pub new_assignments: i64,
    /// Total load after routing
    pub total_load: i64,
    /// Capacity
    pub capacity: i64,
    /// Utilization percentage
    pub utilization_pct: f64,
}

/// Routing statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoutingStats {
    /// Total leads to route
    pub total_leads: usize,
    /// Successfully assigned leads
    pub assigned_leads: usize,
    /// Leads that could not be assigned
    pub unassigned_leads: usize,
    /// Average fit score of assignments
    pub average_fit_score: f64,
    /// Total estimated value of assigned leads
    pub total_estimated_value: f64,
    /// Summary message
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_lead(id: &str, territory: &str) -> Lead {
        Lead {
            id: id.to_string(),
            score: 75.0,
            territory: territory.to_string(),
            segment: "enterprise".to_string(),
            required_skills: vec![],
            estimated_value: 50000.0,
            priority: 3,
        }
    }

    fn create_test_rep(id: &str, territories: Vec<&str>) -> SalesRep {
        SalesRep {
            id: id.to_string(),
            name: format!("Rep {}", id),
            capacity: 10,
            current_load: 5,
            territories: territories.iter().map(|s| s.to_string()).collect(),
            segments: vec!["enterprise".to_string()],
            skills: vec!["negotiation".to_string()],
            performance_score: 80.0,
        }
    }

    #[test]
    fn test_rep_available_capacity() {
        let rep = create_test_rep("r1", vec!["west"]);
        assert_eq!(rep.available_capacity(), 5);
    }

    #[test]
    fn test_rep_covers_territory() {
        let rep = create_test_rep("r1", vec!["west", "midwest"]);
        assert!(rep.covers_territory("west"));
        assert!(rep.covers_territory("midwest"));
        assert!(!rep.covers_territory("east"));
    }

    #[test]
    fn test_wildcard_territory() {
        let rep = SalesRep {
            id: "r1".to_string(),
            name: "Global Rep".to_string(),
            capacity: 20,
            current_load: 0,
            territories: vec!["*".to_string()],
            segments: vec![],
            skills: vec![],
            performance_score: 50.0,
        };
        assert!(rep.covers_territory("anywhere"));
    }

    #[test]
    fn test_lead_fit_score() {
        let lead = create_test_lead("l1", "west");
        let rep = create_test_rep("r1", vec!["west"]);

        let fit = lead.calculate_fit_score(&rep);
        // Territory: 40, Segment: 30, Skills: 20 (no required), Performance: 8
        assert!(fit > 90.0);
    }

    #[test]
    fn test_input_validation() {
        let mut input = LeadRoutingInput {
            leads: vec![create_test_lead("l1", "west")],
            reps: vec![create_test_rep("r1", vec!["west"])],
            config: RoutingConfig::default(),
        };

        assert!(input.validate().is_ok());

        input.leads.clear();
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_duplicate_lead_validation() {
        let input = LeadRoutingInput {
            leads: vec![
                create_test_lead("l1", "west"),
                create_test_lead("l1", "east"), // Duplicate ID
            ],
            reps: vec![create_test_rep("r1", vec!["west"])],
            config: RoutingConfig::default(),
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_capacity_exceeded_validation() {
        let input = LeadRoutingInput {
            leads: vec![create_test_lead("l1", "west")],
            reps: vec![SalesRep {
                id: "r1".to_string(),
                name: "Rep 1".to_string(),
                capacity: 5,
                current_load: 10, // Exceeds capacity
                territories: vec!["west".to_string()],
                segments: vec![],
                skills: vec![],
                performance_score: 50.0,
            }],
            config: RoutingConfig::default(),
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_skills_requirement() {
        let lead = Lead {
            id: "l1".to_string(),
            score: 80.0,
            territory: "west".to_string(),
            segment: "enterprise".to_string(),
            required_skills: vec!["cloud".to_string(), "ai".to_string()],
            estimated_value: 100000.0,
            priority: 1,
        };

        let rep_with_skills = SalesRep {
            id: "r1".to_string(),
            name: "Rep 1".to_string(),
            capacity: 10,
            current_load: 0,
            territories: vec!["west".to_string()],
            segments: vec!["enterprise".to_string()],
            skills: vec!["cloud".to_string(), "ai".to_string(), "ml".to_string()],
            performance_score: 90.0,
        };

        let rep_without_skills = SalesRep {
            id: "r2".to_string(),
            name: "Rep 2".to_string(),
            capacity: 10,
            current_load: 0,
            territories: vec!["west".to_string()],
            segments: vec!["enterprise".to_string()],
            skills: vec!["cloud".to_string()], // Missing "ai"
            performance_score: 90.0,
        };

        assert!(lead.rep_has_required_skills(&rep_with_skills));
        assert!(!lead.rep_has_required_skills(&rep_without_skills));
    }
}
