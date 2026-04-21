//! Solver for Vendor Shortlist pack

use super::types::*;
use converge_pack::PackSolver;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{ProblemSpec, ReplayEnvelope, SolverReport, StopReason};

/// Score-based ranking solver for vendor shortlisting
///
/// Algorithm:
/// 1. Filter vendors by compliance and certifications
/// 2. Filter by minimum score and maximum risk
/// 3. Calculate composite score for each vendor
/// 4. Rank by composite score, take top N
pub struct ScoreRankingSolver;

impl ScoreRankingSolver {
    /// Solve the vendor shortlist problem
    pub fn solve_shortlist(
        &self,
        input: &VendorShortlistInput,
        spec: &ProblemSpec,
    ) -> Result<(VendorShortlistOutput, SolverReport)> {
        let seed = spec.seed();
        let reqs = &input.requirements;

        let mut shortlist = Vec::new();
        let mut rejected = Vec::new();

        // Evaluate each vendor
        for vendor in &input.vendors {
            // Check compliance
            if !vendor.is_compliant() {
                rejected.push(RejectedVendor {
                    vendor_id: vendor.id.clone(),
                    vendor_name: vendor.name.clone(),
                    reason: format!("Non-compliant status: {}", vendor.compliance_status),
                });
                continue;
            }

            // Check certifications
            if !vendor.has_certifications(&reqs.required_certifications) {
                let missing: Vec<_> = reqs
                    .required_certifications
                    .iter()
                    .filter(|c| !vendor.certifications.contains(c))
                    .collect();
                rejected.push(RejectedVendor {
                    vendor_id: vendor.id.clone(),
                    vendor_name: vendor.name.clone(),
                    reason: format!(
                        "Missing certifications: {}",
                        missing
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                });
                continue;
            }

            // Check minimum score
            if vendor.score < reqs.min_score {
                rejected.push(RejectedVendor {
                    vendor_id: vendor.id.clone(),
                    vendor_name: vendor.name.clone(),
                    reason: format!(
                        "Score {:.1} below minimum {:.1}",
                        vendor.score, reqs.min_score
                    ),
                });
                continue;
            }

            // Check risk threshold
            if vendor.risk_score > reqs.max_risk_score {
                rejected.push(RejectedVendor {
                    vendor_id: vendor.id.clone(),
                    vendor_name: vendor.name.clone(),
                    reason: format!(
                        "Risk score {:.1} exceeds maximum {:.1}",
                        vendor.risk_score, reqs.max_risk_score
                    ),
                });
                continue;
            }

            // Vendor passes all checks
            shortlist.push((vendor, vendor.composite_score()));
        }

        // Sort by composite score descending
        shortlist.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Apply tie-breaking for equal scores
        let tie_break = &spec.determinism.tie_break;

        // Group by composite score and apply tie-breaking
        let mut final_list: Vec<(&Vendor, f64)> = Vec::new();
        let mut current_score = f64::INFINITY;
        let mut score_group: Vec<(&Vendor, f64)> = vec![];

        for (vendor, score) in shortlist {
            if (score - current_score).abs() < 0.01 {
                score_group.push((vendor, score));
            } else {
                if !score_group.is_empty() {
                    // Sort by ID for deterministic tie-breaking
                    score_group.sort_by(|a, b| a.0.id.cmp(&b.0.id));
                    if let Some(selected) =
                        tie_break.select_by(&score_group, seed, |a, b| a.0.id.cmp(&b.0.id))
                    {
                        final_list.push(*selected);
                    } else {
                        final_list.extend(score_group.drain(..));
                    }
                }
                score_group = vec![(vendor, score)];
                current_score = score;
            }
        }
        // Don't forget the last group
        if !score_group.is_empty() {
            score_group.sort_by(|a, b| a.0.id.cmp(&b.0.id));
            if let Some(selected) =
                tie_break.select_by(&score_group, seed, |a, b| a.0.id.cmp(&b.0.id))
            {
                final_list.push(*selected);
            } else {
                final_list.extend(score_group.drain(..));
            }
        }

        // Take top N
        let top_n: Vec<_> = final_list.into_iter().take(reqs.max_vendors).collect();

        // Build output
        let shortlisted: Vec<ShortlistedVendor> = top_n
            .iter()
            .enumerate()
            .map(|(i, (vendor, composite))| ShortlistedVendor {
                vendor_id: vendor.id.clone(),
                vendor_name: vendor.name.clone(),
                rank: i + 1,
                score: vendor.score,
                composite_score: *composite,
            })
            .collect();

        let total_shortlisted = shortlisted.len();
        let average_score = if total_shortlisted > 0 {
            shortlisted.iter().map(|v| v.score).sum::<f64>() / total_shortlisted as f64
        } else {
            0.0
        };

        let output = VendorShortlistOutput {
            shortlist: shortlisted,
            rejected,
            stats: ShortlistStats {
                total_evaluated: input.vendors.len(),
                total_shortlisted,
                total_rejected: input.vendors.len() - total_shortlisted,
                average_score,
                reason: if total_shortlisted > 0 {
                    format!(
                        "Selected top {} vendors by composite score",
                        total_shortlisted
                    )
                } else {
                    "No vendors met all requirements".to_string()
                },
            },
        };

        let replay = ReplayEnvelope::minimal(seed);
        let report = if total_shortlisted > 0 {
            SolverReport::optimal("score-rank-v1", average_score, replay)
        } else {
            SolverReport::infeasible("score-rank-v1", vec![], StopReason::NoFeasible, replay)
        };

        Ok((output, report))
    }
}

impl PackSolver for ScoreRankingSolver {
    fn id(&self) -> &'static str {
        "score-rank-v1"
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<(serde_json::Value, SolverReport)> {
        let input: VendorShortlistInput = spec.inputs_as()?;
        let (output, report) = self.solve_shortlist(&input, spec)?;
        let json = serde_json::to_value(&output)
            .map_err(|e| converge_pack::GateError::invalid_input(e.to_string()))?;
        Ok((json, report))
    }

    fn is_exact(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::ObjectiveSpec;

    fn create_test_input() -> VendorShortlistInput {
        VendorShortlistInput {
            vendors: vec![
                Vendor {
                    id: "v1".to_string(),
                    name: "Acme Corp".to_string(),
                    score: 85.0,
                    risk_score: 20.0,
                    compliance_status: "compliant".to_string(),
                    certifications: vec!["ISO9001".to_string(), "SOC2".to_string()],
                },
                Vendor {
                    id: "v2".to_string(),
                    name: "BetaCo".to_string(),
                    score: 75.0,
                    risk_score: 15.0,
                    compliance_status: "compliant".to_string(),
                    certifications: vec!["ISO9001".to_string()],
                },
                Vendor {
                    id: "v3".to_string(),
                    name: "GammaTech".to_string(),
                    score: 90.0,
                    risk_score: 60.0, // High risk
                    compliance_status: "compliant".to_string(),
                    certifications: vec!["ISO9001".to_string()],
                },
            ],
            requirements: ShortlistRequirements {
                max_vendors: 2,
                min_score: 70.0,
                max_risk_score: 50.0,
                required_certifications: vec!["ISO9001".to_string()],
            },
        }
    }

    fn create_spec(input: &VendorShortlistInput, seed: u64) -> ProblemSpec {
        ProblemSpec::builder("test", "tenant")
            .objective(ObjectiveSpec::maximize("score"))
            .inputs(input)
            .unwrap()
            .seed(seed)
            .build()
            .unwrap()
    }

    #[test]
    fn test_shortlist_ranking() {
        let solver = ScoreRankingSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, report) = solver.solve_shortlist(&input, &spec).unwrap();

        assert_eq!(output.shortlist.len(), 2);
        assert!(report.feasible);

        // v1 should be first (85 - 10 = 75), v2 second (75 - 7.5 = 67.5)
        assert_eq!(output.shortlist[0].vendor_id, "v1");
        assert_eq!(output.shortlist[0].rank, 1);
    }

    #[test]
    fn test_risk_filtering() {
        let solver = ScoreRankingSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, _) = solver.solve_shortlist(&input, &spec).unwrap();

        // v3 should be rejected due to high risk
        let v3_rejected = output.rejected.iter().find(|r| r.vendor_id == "v3");
        assert!(v3_rejected.is_some());
        assert!(v3_rejected.unwrap().reason.contains("Risk score"));
    }

    #[test]
    fn test_certification_filtering() {
        let solver = ScoreRankingSolver;
        let mut input = create_test_input();
        input.requirements.required_certifications = vec!["SOC2".to_string()];

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_shortlist(&input, &spec).unwrap();

        // Only v1 has SOC2
        assert_eq!(output.shortlist.len(), 1);
        assert_eq!(output.shortlist[0].vendor_id, "v1");
    }

    #[test]
    fn test_no_qualifying_vendors() {
        let solver = ScoreRankingSolver;
        let mut input = create_test_input();
        input.requirements.min_score = 100.0; // No one meets this

        let spec = create_spec(&input, 42);
        let (output, report) = solver.solve_shortlist(&input, &spec).unwrap();

        assert!(output.shortlist.is_empty());
        assert!(!report.feasible);
    }

    #[test]
    fn test_determinism() {
        let solver = ScoreRankingSolver;
        let input = create_test_input();

        let spec1 = create_spec(&input, 12345);
        let spec2 = create_spec(&input, 12345);

        let (output1, _) = solver.solve_shortlist(&input, &spec1).unwrap();
        let (output2, _) = solver.solve_shortlist(&input, &spec2).unwrap();

        assert_eq!(output1.shortlist.len(), output2.shortlist.len());
        for (a, b) in output1.shortlist.iter().zip(output2.shortlist.iter()) {
            assert_eq!(a.vendor_id, b.vendor_id);
        }
    }
}
