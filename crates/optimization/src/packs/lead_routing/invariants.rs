//! Invariants for Lead Routing pack

use super::types::{LeadRoutingInput, LeadRoutingOutput};
use crate::packs::{InvariantDef, InvariantResult};
use converge_pack::gate::Violation;

/// Get invariant definitions
pub fn get_invariants() -> Vec<InvariantDef> {
    vec![
        InvariantDef::critical("all_leads_assigned", "All leads must be assigned to a rep"),
        InvariantDef::critical(
            "capacity_not_exceeded",
            "No rep should exceed their capacity",
        ),
        InvariantDef::critical(
            "territory_respected",
            "Leads should be assigned to reps covering their territory",
        ),
        InvariantDef::advisory(
            "load_balanced",
            "Lead assignments should be reasonably balanced across reps",
        ),
        InvariantDef::advisory(
            "fit_score_acceptable",
            "Average fit score should meet minimum threshold",
        ),
    ]
}

/// Invariant definitions constant (for static reference)
pub const INVARIANTS: &[InvariantDef] = &[];

/// Check all invariants
pub fn check_all_invariants(
    output: &LeadRoutingOutput,
    input: &LeadRoutingInput,
) -> Vec<InvariantResult> {
    vec![
        check_all_leads_assigned(output),
        check_capacity_not_exceeded(output, input),
        check_territory_respected(output, input),
        check_load_balanced(output, input),
        check_fit_score_acceptable(output),
    ]
}

/// Check that all leads are assigned (critical)
fn check_all_leads_assigned(output: &LeadRoutingOutput) -> InvariantResult {
    let invariant = "all_leads_assigned";

    if output.unassigned.is_empty() {
        InvariantResult::pass(invariant)
    } else {
        let unassigned_ids: Vec<_> = output
            .unassigned
            .iter()
            .map(|u| u.lead_id.as_str())
            .take(5) // Limit to first 5 for readability
            .collect();

        let message = if output.unassigned.len() > 5 {
            format!(
                "{} leads unassigned: {}, ... and {} more",
                output.unassigned.len(),
                unassigned_ids.join(", "),
                output.unassigned.len() - 5
            )
        } else {
            format!(
                "{} leads unassigned: {}",
                output.unassigned.len(),
                unassigned_ids.join(", ")
            )
        };

        let severity = (output.unassigned.len() as f64 / output.stats.total_leads as f64).min(1.0);

        let violation = Violation::new(invariant, severity, message);
        InvariantResult::fail(invariant, violation)
    }
}

/// Check that no rep exceeds capacity (critical)
fn check_capacity_not_exceeded(
    output: &LeadRoutingOutput,
    _input: &LeadRoutingInput,
) -> InvariantResult {
    let invariant = "capacity_not_exceeded";

    let violations: Vec<String> = output
        .rep_utilization
        .iter()
        .filter(|u| u.total_load > u.capacity)
        .map(|u| {
            format!(
                "{} has {} leads but capacity is {}",
                u.rep_name, u.total_load, u.capacity
            )
        })
        .collect();

    if violations.is_empty() {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(invariant, 1.0, violations.join("; "));
        InvariantResult::fail(invariant, violation)
    }
}

/// Check that territory assignments are respected (critical when required)
fn check_territory_respected(
    output: &LeadRoutingOutput,
    input: &LeadRoutingInput,
) -> InvariantResult {
    let invariant = "territory_respected";

    // Only critical if territory matching is required
    if !input.config.require_territory_match {
        return InvariantResult::pass(invariant);
    }

    let mut mismatches = Vec::new();

    for assignment in &output.assignments {
        // Find the lead and rep
        let lead = input.leads.iter().find(|l| l.id == assignment.lead_id);
        let rep = input.reps.iter().find(|r| r.id == assignment.rep_id);

        if let (Some(lead), Some(rep)) = (lead, rep) {
            if !rep.covers_territory(&lead.territory) {
                mismatches.push(format!(
                    "Lead {} ({}) assigned to {} who covers {:?}",
                    lead.id, lead.territory, rep.name, rep.territories
                ));
            }
        }
    }

    if mismatches.is_empty() {
        InvariantResult::pass(invariant)
    } else {
        let message = if mismatches.len() > 3 {
            format!(
                "{} territory mismatches: {}, ... and {} more",
                mismatches.len(),
                mismatches[..3].join("; "),
                mismatches.len() - 3
            )
        } else {
            format!(
                "{} territory mismatches: {}",
                mismatches.len(),
                mismatches.join("; ")
            )
        };

        let violation = Violation::new(invariant, 1.0, message);
        InvariantResult::fail(invariant, violation)
    }
}

/// Check that load is reasonably balanced across reps (advisory)
fn check_load_balanced(output: &LeadRoutingOutput, _input: &LeadRoutingInput) -> InvariantResult {
    let invariant = "load_balanced";

    if output.rep_utilization.len() < 2 {
        return InvariantResult::pass(invariant);
    }

    // Calculate utilization variance
    let utilizations: Vec<f64> = output
        .rep_utilization
        .iter()
        .map(|u| u.utilization_pct)
        .collect();

    let avg_util: f64 = utilizations.iter().sum::<f64>() / utilizations.len() as f64;
    let variance: f64 = utilizations
        .iter()
        .map(|u| (u - avg_util).powi(2))
        .sum::<f64>()
        / utilizations.len() as f64;
    let std_dev = variance.sqrt();

    // Allow 25% standard deviation as acceptable
    let threshold = 25.0;

    if std_dev <= threshold {
        InvariantResult::pass(invariant)
    } else {
        let min_util = utilizations.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_util = utilizations
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        let message = format!(
            "Load imbalance detected: utilization ranges from {:.1}% to {:.1}% (std dev: {:.1}%)",
            min_util, max_util, std_dev
        );

        // Severity based on how much we exceed threshold
        let severity = ((std_dev - threshold) / threshold).min(1.0).max(0.1);

        let violation = Violation::new(invariant, severity, message);
        InvariantResult::fail(invariant, violation)
    }
}

/// Check that average fit score meets minimum threshold (advisory)
fn check_fit_score_acceptable(output: &LeadRoutingOutput) -> InvariantResult {
    let invariant = "fit_score_acceptable";

    if output.assignments.is_empty() {
        return InvariantResult::pass(invariant);
    }

    let min_acceptable_score = 30.0;
    let avg_score = output.stats.average_fit_score;

    if avg_score >= min_acceptable_score {
        InvariantResult::pass(invariant)
    } else {
        let message = format!(
            "Average fit score {:.1} is below acceptable threshold {:.1}",
            avg_score, min_acceptable_score
        );

        let severity = ((min_acceptable_score - avg_score) / min_acceptable_score)
            .min(1.0)
            .max(0.1);

        let violation = Violation::new(invariant, severity, message);
        InvariantResult::fail(invariant, violation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::lead_routing::types::*;

    fn create_valid_output() -> LeadRoutingOutput {
        LeadRoutingOutput {
            assignments: vec![
                LeadAssignment {
                    lead_id: "lead-1".to_string(),
                    rep_id: "rep-1".to_string(),
                    rep_name: "Alice".to_string(),
                    fit_score: 85.0,
                    scoring_rationale: ScoringRationale {
                        territory_score: 100.0,
                        segment_score: 100.0,
                        skills_score: 80.0,
                        performance_score: 90.0,
                        capacity_factor: 0.9,
                        explanation: "territory match, segment match".to_string(),
                    },
                },
                LeadAssignment {
                    lead_id: "lead-2".to_string(),
                    rep_id: "rep-2".to_string(),
                    rep_name: "Bob".to_string(),
                    fit_score: 75.0,
                    scoring_rationale: ScoringRationale {
                        territory_score: 100.0,
                        segment_score: 100.0,
                        skills_score: 60.0,
                        performance_score: 75.0,
                        capacity_factor: 0.95,
                        explanation: "territory match".to_string(),
                    },
                },
            ],
            unassigned: vec![],
            rep_utilization: vec![
                RepUtilization {
                    rep_id: "rep-1".to_string(),
                    rep_name: "Alice".to_string(),
                    new_assignments: 1,
                    total_load: 8,
                    capacity: 10,
                    utilization_pct: 80.0,
                },
                RepUtilization {
                    rep_id: "rep-2".to_string(),
                    rep_name: "Bob".to_string(),
                    new_assignments: 1,
                    total_load: 6,
                    capacity: 8,
                    utilization_pct: 75.0,
                },
            ],
            stats: RoutingStats {
                total_leads: 2,
                assigned_leads: 2,
                unassigned_leads: 0,
                average_fit_score: 80.0,
                total_estimated_value: 150000.0,
                summary: "All leads assigned".to_string(),
            },
        }
    }

    fn create_valid_input() -> LeadRoutingInput {
        LeadRoutingInput {
            leads: vec![
                Lead {
                    id: "lead-1".to_string(),
                    score: 80.0,
                    territory: "west".to_string(),
                    segment: "enterprise".to_string(),
                    required_skills: vec![],
                    estimated_value: 100000.0,
                    priority: 1,
                },
                Lead {
                    id: "lead-2".to_string(),
                    score: 70.0,
                    territory: "east".to_string(),
                    segment: "smb".to_string(),
                    required_skills: vec![],
                    estimated_value: 50000.0,
                    priority: 2,
                },
            ],
            reps: vec![
                SalesRep {
                    id: "rep-1".to_string(),
                    name: "Alice".to_string(),
                    capacity: 10,
                    current_load: 7,
                    territories: vec!["west".to_string()],
                    segments: vec!["enterprise".to_string()],
                    skills: vec![],
                    performance_score: 90.0,
                },
                SalesRep {
                    id: "rep-2".to_string(),
                    name: "Bob".to_string(),
                    capacity: 8,
                    current_load: 5,
                    territories: vec!["east".to_string()],
                    segments: vec!["smb".to_string()],
                    skills: vec![],
                    performance_score: 75.0,
                },
            ],
            config: RoutingConfig::default(),
        }
    }

    #[test]
    fn test_all_pass_valid_output() {
        let output = create_valid_output();
        let input = create_valid_input();
        let results = check_all_invariants(&output, &input);

        for result in &results {
            assert!(
                result.passed,
                "Invariant {} failed: {:?}",
                result.invariant, result.violation
            );
        }
    }

    #[test]
    fn test_unassigned_leads_fail() {
        let mut output = create_valid_output();
        output.unassigned = vec![UnassignedLead {
            lead_id: "lead-3".to_string(),
            reason: "No capacity".to_string(),
        }];
        output.stats.unassigned_leads = 1;
        output.stats.total_leads = 3;

        let result = check_all_leads_assigned(&output);
        assert!(!result.passed);
        assert!(
            result
                .violation
                .as_ref()
                .unwrap()
                .explanation
                .contains("lead-3")
        );
    }

    #[test]
    fn test_capacity_exceeded_fail() {
        let mut output = create_valid_output();
        output.rep_utilization[0].total_load = 15;
        output.rep_utilization[0].capacity = 10;

        let input = create_valid_input();
        let result = check_capacity_not_exceeded(&output, &input);

        assert!(!result.passed);
        assert_eq!(result.violation.as_ref().unwrap().severity, 1.0);
    }

    #[test]
    fn test_territory_mismatch_with_requirement() {
        let output = create_valid_output();
        let mut input = create_valid_input();

        // Enable territory requirement and create mismatch
        input.config.require_territory_match = true;
        input.leads[0].territory = "south".to_string(); // Rep doesn't cover south

        let result = check_territory_respected(&output, &input);
        assert!(!result.passed);
    }

    #[test]
    fn test_territory_mismatch_without_requirement() {
        let output = create_valid_output();
        let mut input = create_valid_input();

        // Territory matching not required
        input.config.require_territory_match = false;
        input.leads[0].territory = "south".to_string();

        let result = check_territory_respected(&output, &input);
        assert!(result.passed);
    }

    #[test]
    fn test_load_imbalanced() {
        let mut output = create_valid_output();

        // Create significant imbalance
        output.rep_utilization[0].utilization_pct = 95.0;
        output.rep_utilization[1].utilization_pct = 25.0;

        let input = create_valid_input();
        let result = check_load_balanced(&output, &input);

        assert!(!result.passed);
        assert!(result.violation.as_ref().unwrap().severity < 1.0); // Advisory
    }

    #[test]
    fn test_low_fit_score() {
        let mut output = create_valid_output();
        output.stats.average_fit_score = 15.0;

        let result = check_fit_score_acceptable(&output);
        assert!(!result.passed);
        assert!(result.violation.as_ref().unwrap().severity < 1.0); // Advisory
    }

    #[test]
    fn test_get_invariants() {
        let invariants = get_invariants();

        assert_eq!(invariants.len(), 5);

        // Check critical invariants
        let critical: Vec<_> = invariants.iter().filter(|i| i.critical).collect();
        assert_eq!(critical.len(), 3);

        // Check advisory invariants
        let advisory: Vec<_> = invariants.iter().filter(|i| !i.critical).collect();
        assert_eq!(advisory.len(), 2);
    }
}
