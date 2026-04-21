//! Invariants for Anomaly Triage pack

use super::types::AnomalyTriageOutput;
use crate::packs::{InvariantDef, InvariantResult};
use converge_pack::gate::Violation;

/// Get invariant definitions
pub fn get_invariants() -> Vec<InvariantDef> {
    vec![
        InvariantDef::critical(
            "all_anomalies_triaged",
            "All input anomalies must be triaged",
        ),
        InvariantDef::critical(
            "priorities_sequential",
            "Priorities must be sequential starting at 1",
        ),
        InvariantDef::advisory(
            "critical_escalated",
            "Critical anomalies should be escalated",
        ),
        InvariantDef::advisory(
            "recommendations_provided",
            "All triaged anomalies should have recommendations",
        ),
    ]
}

/// Invariant definitions constant
pub const INVARIANTS: &[InvariantDef] = &[];

/// Check all invariants
pub fn check_all_invariants(output: &AnomalyTriageOutput) -> Vec<InvariantResult> {
    vec![
        check_all_anomalies_triaged(output),
        check_priorities_sequential(output),
        check_critical_escalated(output),
        check_recommendations_provided(output),
    ]
}

fn check_all_anomalies_triaged(output: &AnomalyTriageOutput) -> InvariantResult {
    let invariant = "all_anomalies_triaged";

    // This would need input count to validate fully
    // For now, just check that we have results if we're supposed to
    if !output.triaged.is_empty()
        || (output.severity_summary.critical == 0
            && output.severity_summary.high == 0
            && output.severity_summary.medium == 0
            && output.severity_summary.low == 0)
    {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            1.0,
            "Mismatch between summary counts and triaged items",
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_priorities_sequential(output: &AnomalyTriageOutput) -> InvariantResult {
    let invariant = "priorities_sequential";

    if output.triaged.is_empty() {
        return InvariantResult::pass(invariant);
    }

    for (i, item) in output.triaged.iter().enumerate() {
        if item.priority != i + 1 {
            let violation = Violation::new(
                invariant,
                1.0,
                format!(
                    "Anomaly {} has priority {} but should be {}",
                    item.anomaly_id,
                    item.priority,
                    i + 1
                ),
            );
            return InvariantResult::fail(invariant, violation);
        }
    }

    InvariantResult::pass(invariant)
}

fn check_critical_escalated(output: &AnomalyTriageOutput) -> InvariantResult {
    let invariant = "critical_escalated";

    let unescalated_critical: Vec<_> = output
        .triaged
        .iter()
        .filter(|t| t.severity == "critical" && !t.escalate)
        .collect();

    if unescalated_critical.is_empty() {
        InvariantResult::pass(invariant)
    } else {
        let ids: Vec<_> = unescalated_critical
            .iter()
            .map(|t| t.anomaly_id.as_str())
            .collect();
        let violation = Violation::new(
            invariant,
            0.7,
            format!("Critical anomalies not escalated: {}", ids.join(", ")),
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_recommendations_provided(output: &AnomalyTriageOutput) -> InvariantResult {
    let invariant = "recommendations_provided";

    let missing_recommendations: Vec<_> = output
        .triaged
        .iter()
        .filter(|t| t.recommended_actions.is_empty())
        .collect();

    if missing_recommendations.is_empty() {
        InvariantResult::pass(invariant)
    } else {
        let ids: Vec<_> = missing_recommendations
            .iter()
            .map(|t| t.anomaly_id.as_str())
            .collect();
        let violation = Violation::new(
            invariant,
            0.3,
            format!("Anomalies without recommendations: {}", ids.join(", ")),
        );
        InvariantResult::fail(invariant, violation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::anomaly_triage::types::*;

    fn create_valid_output() -> AnomalyTriageOutput {
        AnomalyTriageOutput {
            triaged: vec![
                TriagedAnomaly {
                    anomaly_id: "a1".to_string(),
                    severity: "critical".to_string(),
                    priority: 1,
                    escalate: true,
                    reason: "z-score 5.2 exceeds critical threshold".to_string(),
                    recommended_actions: vec!["Investigate immediately".to_string()],
                },
                TriagedAnomaly {
                    anomaly_id: "a2".to_string(),
                    severity: "high".to_string(),
                    priority: 2,
                    escalate: false,
                    reason: "z-score 3.1 exceeds high threshold".to_string(),
                    recommended_actions: vec!["Review within 1 hour".to_string()],
                },
            ],
            escalation_count: 1,
            severity_summary: SeveritySummary {
                critical: 1,
                high: 1,
                medium: 0,
                low: 0,
            },
        }
    }

    #[test]
    fn test_all_pass_valid_output() {
        let output = create_valid_output();
        let results = check_all_invariants(&output);

        for result in &results {
            assert!(result.passed, "Invariant {} failed", result.invariant);
        }
    }

    #[test]
    fn test_invalid_priority_sequence() {
        let mut output = create_valid_output();
        output.triaged[1].priority = 5; // Should be 2

        let result = check_priorities_sequential(&output);
        assert!(!result.passed);
    }

    #[test]
    fn test_unescalated_critical() {
        let mut output = create_valid_output();
        output.triaged[0].escalate = false;

        let result = check_critical_escalated(&output);
        assert!(!result.passed);
        assert!(result.violation.as_ref().unwrap().severity < 1.0); // Advisory
    }

    #[test]
    fn test_missing_recommendations() {
        let mut output = create_valid_output();
        output.triaged[0].recommended_actions = vec![];

        let result = check_recommendations_provided(&output);
        assert!(!result.passed);
    }
}
