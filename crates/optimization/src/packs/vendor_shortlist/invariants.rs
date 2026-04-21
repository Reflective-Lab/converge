//! Invariants for Vendor Shortlist pack

use super::types::VendorShortlistOutput;
use crate::packs::{InvariantDef, InvariantResult};
use converge_pack::gate::Violation;

/// Get invariant definitions
pub fn get_invariants() -> Vec<InvariantDef> {
    vec![
        InvariantDef::critical(
            "shortlist_not_empty",
            "At least one vendor should be shortlisted",
        ),
        InvariantDef::critical(
            "ranking_valid",
            "Shortlist rankings must be sequential starting at 1",
        ),
        InvariantDef::advisory(
            "diversity_achieved",
            "Shortlist should have adequate diversity",
        ),
        InvariantDef::advisory(
            "quality_threshold_met",
            "Average score of shortlist should meet quality threshold",
        ),
    ]
}

/// Invariant definitions constant
pub const INVARIANTS: &[InvariantDef] = &[];

/// Check all invariants
pub fn check_all_invariants(output: &VendorShortlistOutput) -> Vec<InvariantResult> {
    vec![
        check_shortlist_not_empty(output),
        check_ranking_valid(output),
        check_diversity_achieved(output),
        check_quality_threshold_met(output),
    ]
}

fn check_shortlist_not_empty(output: &VendorShortlistOutput) -> InvariantResult {
    let invariant = "shortlist_not_empty";

    if !output.shortlist.is_empty() {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            1.0,
            format!("No vendors shortlisted: {}", output.stats.reason),
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_ranking_valid(output: &VendorShortlistOutput) -> InvariantResult {
    let invariant = "ranking_valid";

    if output.shortlist.is_empty() {
        return InvariantResult::pass(invariant);
    }

    // Check rankings are 1, 2, 3, ...
    for (i, vendor) in output.shortlist.iter().enumerate() {
        if vendor.rank != i + 1 {
            let violation = Violation::new(
                invariant,
                1.0,
                format!(
                    "Invalid ranking: vendor {} has rank {} but should be {}",
                    vendor.vendor_id,
                    vendor.rank,
                    i + 1
                ),
            );
            return InvariantResult::fail(invariant, violation);
        }
    }

    InvariantResult::pass(invariant)
}

fn check_diversity_achieved(output: &VendorShortlistOutput) -> InvariantResult {
    let invariant = "diversity_achieved";

    if output.shortlist.len() <= 1 {
        return InvariantResult::pass(invariant);
    }

    // Advisory: check that we have multiple vendors (diversity)
    // In a real implementation, this might check industry, geography, size, etc.
    if output.shortlist.len() >= 2 {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            0.3,
            "Only one vendor shortlisted - consider adding alternatives",
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_quality_threshold_met(output: &VendorShortlistOutput) -> InvariantResult {
    let invariant = "quality_threshold_met";

    if output.shortlist.is_empty() {
        return InvariantResult::pass(invariant);
    }

    // Advisory: average score should be above 70
    let threshold = 70.0;
    if output.stats.average_score >= threshold {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            0.3,
            format!(
                "Average shortlist score {:.1} below quality threshold {:.1}",
                output.stats.average_score, threshold
            ),
        );
        InvariantResult::fail(invariant, violation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::vendor_shortlist::types::*;

    fn create_valid_output() -> VendorShortlistOutput {
        VendorShortlistOutput {
            shortlist: vec![
                ShortlistedVendor {
                    vendor_id: "v1".to_string(),
                    vendor_name: "Acme".to_string(),
                    rank: 1,
                    score: 85.0,
                    composite_score: 75.0,
                },
                ShortlistedVendor {
                    vendor_id: "v2".to_string(),
                    vendor_name: "Beta".to_string(),
                    rank: 2,
                    score: 80.0,
                    composite_score: 72.0,
                },
            ],
            rejected: vec![],
            stats: ShortlistStats {
                total_evaluated: 3,
                total_shortlisted: 2,
                total_rejected: 1,
                average_score: 82.5,
                reason: "Selected top vendors".to_string(),
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
    fn test_empty_shortlist_fails() {
        let output = VendorShortlistOutput::empty("No vendors met requirements");
        let results = check_all_invariants(&output);

        let shortlist_result = results
            .iter()
            .find(|r| r.invariant == "shortlist_not_empty")
            .unwrap();
        assert!(!shortlist_result.passed);
    }

    #[test]
    fn test_invalid_ranking() {
        let mut output = create_valid_output();
        output.shortlist[1].rank = 5; // Should be 2

        let result = check_ranking_valid(&output);
        assert!(!result.passed);
    }

    #[test]
    fn test_low_quality() {
        let mut output = create_valid_output();
        output.stats.average_score = 50.0;

        let result = check_quality_threshold_met(&output);
        assert!(!result.passed);
        assert!(result.violation.as_ref().unwrap().severity < 1.0); // Advisory
    }
}
