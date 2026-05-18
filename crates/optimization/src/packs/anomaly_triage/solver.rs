//! Solver for Anomaly Triage pack

use super::types::*;
use converge_pack::PackSolver;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{ProblemSpec, ReplayEnvelope, SolverReport, StopReason};

/// Threshold-based solver for anomaly triage
///
/// Algorithm:
/// 1. Classify each anomaly by z-score threshold
/// 2. Sort by severity then by z-score (most severe first)
/// 3. Apply escalation policies
/// 4. Assign priorities
pub struct ThresholdSolver;

impl ThresholdSolver {
    /// Solve the anomaly triage problem
    pub fn solve_triage(
        &self,
        input: &AnomalyTriageInput,
        spec: &ProblemSpec,
    ) -> Result<(AnomalyTriageOutput, SolverReport)> {
        let seed = spec.seed();

        if input.anomalies.is_empty() {
            let output = AnomalyTriageOutput::empty();
            let replay = ReplayEnvelope::minimal(seed);
            let report = SolverReport::feasible("threshold-v1", 0.0, StopReason::Feasible, replay);
            return Ok((output, report));
        }

        // Classify and sort anomalies
        let mut classified: Vec<_> = input
            .anomalies
            .iter()
            .map(|a| {
                let severity = a.classify_severity(&input.thresholds);
                (a, severity, a.z_score.abs())
            })
            .collect();

        // Sort by severity (critical > high > medium > low) then by z-score descending
        classified.sort_by(|a, b| {
            let severity_order = |s: &str| match s {
                "critical" => 0,
                "high" => 1,
                "medium" => 2,
                _ => 3,
            };

            let ord = severity_order(a.1).cmp(&severity_order(b.1));
            if ord == std::cmp::Ordering::Equal {
                b.2.total_cmp(&a.2).then_with(|| a.0.id.cmp(&b.0.id))
            } else {
                ord
            }
        });

        // Apply tie-breaking for equal severity and z-score
        let _tie_break = &spec.determinism.tie_break;

        // Group by (severity, z-score) for tie-breaking
        let mut final_order: Vec<(&Anomaly, &str, f64)> = Vec::new();
        let mut current_key = (String::new(), f64::NEG_INFINITY);
        let mut group: Vec<(&Anomaly, &str, f64)> = vec![];

        for (anomaly, severity, z) in classified {
            let key = (severity.to_string(), z);
            if key.0 == current_key.0 && (key.1 - current_key.1).abs() < 0.01 {
                group.push((anomaly, severity, z));
            } else {
                if !group.is_empty() {
                    group.sort_by(|a, b| a.0.id.cmp(&b.0.id));
                    final_order.extend(group.drain(..));
                }
                group = vec![(anomaly, severity, z)];
                current_key = key;
            }
        }
        if !group.is_empty() {
            group.sort_by(|a, b| a.0.id.cmp(&b.0.id));
            final_order.extend(group.drain(..));
        }

        // Build triaged output
        let mut triaged = Vec::new();
        let mut escalation_count = 0;
        let mut summary = SeveritySummary::default();

        for (priority, (anomaly, severity, z)) in final_order.into_iter().enumerate() {
            let policy = input.get_policy(severity);
            let escalate = policy
                .map(|p| p.auto_escalate)
                .unwrap_or(severity == "critical");

            if escalate {
                escalation_count += 1;
            }

            // Update summary
            match severity {
                "critical" => summary.critical += 1,
                "high" => summary.high += 1,
                "medium" => summary.medium += 1,
                _ => summary.low += 1,
            }

            let recommended_actions = self.recommend_actions(severity, &policy);

            triaged.push(TriagedAnomaly {
                anomaly_id: anomaly.id.clone(),
                severity: severity.to_string(),
                priority: priority + 1,
                escalate,
                reason: format!("z-score {:.2} exceeds {} threshold", z, severity),
                recommended_actions,
            });
        }

        let output = AnomalyTriageOutput {
            triaged,
            escalation_count,
            severity_summary: summary,
        };

        let replay = ReplayEnvelope::minimal(seed);
        let report = SolverReport::optimal("threshold-v1", output.escalation_count as f64, replay);

        Ok((output, report))
    }

    fn recommend_actions(&self, severity: &str, policy: &Option<&EscalationPolicy>) -> Vec<String> {
        let mut actions = Vec::new();

        match severity {
            "critical" => {
                actions.push("Immediate investigation required".to_string());
                actions.push("Page on-call engineer".to_string());
            }
            "high" => {
                actions.push("Investigate within 1 hour".to_string());
                actions.push("Create incident ticket".to_string());
            }
            "medium" => {
                actions.push("Review within 4 hours".to_string());
                actions.push("Add to monitoring queue".to_string());
            }
            _ => {
                actions.push("Log for trend analysis".to_string());
            }
        }

        if let Some(p) = policy {
            if !p.notify_channels.is_empty() {
                actions.push(format!("Notify: {}", p.notify_channels.join(", ")));
            }
        }

        actions
    }
}

impl PackSolver for ThresholdSolver {
    fn id(&self) -> &'static str {
        "threshold-v1"
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<(serde_json::Value, SolverReport)> {
        let input: AnomalyTriageInput = spec.inputs_as()?;
        let (output, report) = self.solve_triage(&input, spec)?;
        let json = serde_json::to_value(&output)
            .map_err(|e| converge_pack::GateError::invalid_input(e.to_string()))?;
        Ok((json, report))
    }

    fn is_exact(&self) -> bool {
        true // Deterministic threshold-based classification
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::ObjectiveSpec;

    fn create_test_input() -> AnomalyTriageInput {
        AnomalyTriageInput {
            anomalies: vec![
                Anomaly {
                    id: "a1".to_string(),
                    timestamp: 1700000000,
                    source: "api-server".to_string(),
                    z_score: 5.2,
                    features: serde_json::json!({"metric": "latency"}),
                },
                Anomaly {
                    id: "a2".to_string(),
                    timestamp: 1700000010,
                    source: "database".to_string(),
                    z_score: 3.1,
                    features: serde_json::json!({"metric": "connections"}),
                },
                Anomaly {
                    id: "a3".to_string(),
                    timestamp: 1700000020,
                    source: "cache".to_string(),
                    z_score: 1.5,
                    features: serde_json::json!({"metric": "hit_rate"}),
                },
            ],
            thresholds: SeverityThresholds::default(),
            escalation_policies: vec![EscalationPolicy {
                severity_level: "critical".to_string(),
                auto_escalate: true,
                notify_channels: vec!["pagerduty".to_string(), "slack-oncall".to_string()],
                response_sla_minutes: 15,
            }],
        }
    }

    fn create_spec(input: &AnomalyTriageInput, seed: u64) -> ProblemSpec {
        ProblemSpec::builder("test", "tenant")
            .objective(ObjectiveSpec::minimize("risk"))
            .inputs(input)
            .unwrap()
            .seed(seed)
            .build()
            .unwrap()
    }

    #[test]
    fn test_severity_ordering() {
        let solver = ThresholdSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, report) = solver.solve_triage(&input, &spec).unwrap();

        assert!(report.feasible);
        assert_eq!(output.triaged.len(), 3);

        // Critical should be first
        assert_eq!(output.triaged[0].anomaly_id, "a1");
        assert_eq!(output.triaged[0].severity, "critical");
        assert_eq!(output.triaged[0].priority, 1);

        // High should be second
        assert_eq!(output.triaged[1].anomaly_id, "a2");
        assert_eq!(output.triaged[1].severity, "high");
    }

    #[test]
    fn test_escalation() {
        let solver = ThresholdSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, _) = solver.solve_triage(&input, &spec).unwrap();

        // Only critical should be escalated (based on policy)
        assert_eq!(output.escalation_count, 1);
        assert!(output.triaged[0].escalate);
    }

    #[test]
    fn test_severity_summary() {
        let solver = ThresholdSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, _) = solver.solve_triage(&input, &spec).unwrap();

        assert_eq!(output.severity_summary.critical, 1);
        assert_eq!(output.severity_summary.high, 1);
        assert_eq!(output.severity_summary.low, 1);
    }

    #[test]
    fn test_empty_anomalies() {
        let solver = ThresholdSolver;
        let input = AnomalyTriageInput {
            anomalies: vec![],
            thresholds: SeverityThresholds::default(),
            escalation_policies: vec![],
        };

        let spec = create_spec(&input, 42);
        let (output, report) = solver.solve_triage(&input, &spec).unwrap();

        assert!(output.triaged.is_empty());
        assert!(report.feasible);
    }

    #[test]
    fn test_determinism() {
        let solver = ThresholdSolver;
        let input = create_test_input();

        let spec1 = create_spec(&input, 12345);
        let spec2 = create_spec(&input, 12345);

        let (output1, _) = solver.solve_triage(&input, &spec1).unwrap();
        let (output2, _) = solver.solve_triage(&input, &spec2).unwrap();

        for (a, b) in output1.triaged.iter().zip(output2.triaged.iter()) {
            assert_eq!(a.anomaly_id, b.anomaly_id);
            assert_eq!(a.priority, b.priority);
        }
    }
}
