//! Types for Anomaly Triage pack

use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

/// Input for anomaly triage optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyTriageInput {
    /// Detected anomalies to triage
    pub anomalies: Vec<Anomaly>,
    /// Severity thresholds for classification
    pub thresholds: SeverityThresholds,
    /// Escalation policies
    pub escalation_policies: Vec<EscalationPolicy>,
}

impl AnomalyTriageInput {
    /// Validate the input
    pub fn validate(&self) -> Result<()> {
        if !self.thresholds.critical.is_finite() || self.thresholds.critical < 0.0 {
            return Err(converge_pack::GateError::invalid_input(
                "Critical threshold must be finite and non-negative",
            ));
        }
        if !self.thresholds.high.is_finite() || self.thresholds.high < 0.0 {
            return Err(converge_pack::GateError::invalid_input(
                "High threshold must be finite and non-negative",
            ));
        }
        if !self.thresholds.medium.is_finite() || self.thresholds.medium < 0.0 {
            return Err(converge_pack::GateError::invalid_input(
                "Medium threshold must be finite and non-negative",
            ));
        }
        if self.thresholds.critical <= self.thresholds.high {
            return Err(converge_pack::GateError::invalid_input(
                "Critical threshold must be greater than high threshold",
            ));
        }
        if self.thresholds.high <= self.thresholds.medium {
            return Err(converge_pack::GateError::invalid_input(
                "High threshold must be greater than medium threshold",
            ));
        }
        for anomaly in &self.anomalies {
            if !anomaly.z_score.is_finite() {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Anomaly {} has non-finite z_score",
                    anomaly.id
                )));
            }
        }
        Ok(())
    }

    /// Get anomaly by ID
    pub fn get_anomaly(&self, id: &str) -> Option<&Anomaly> {
        self.anomalies.iter().find(|a| a.id == id)
    }

    /// Get applicable escalation policy for a severity level
    pub fn get_policy(&self, severity: &str) -> Option<&EscalationPolicy> {
        self.escalation_policies
            .iter()
            .find(|p| p.severity_level == severity)
    }
}

/// An anomaly to triage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    /// Anomaly identifier
    pub id: String,
    /// Timestamp of detection (unix epoch)
    pub timestamp: i64,
    /// Source system
    pub source: String,
    /// Statistical z-score indicating deviation
    pub z_score: f64,
    /// Additional features for analysis
    pub features: serde_json::Value,
}

impl Anomaly {
    /// Classify severity based on z-score and thresholds
    pub fn classify_severity(&self, thresholds: &SeverityThresholds) -> &'static str {
        let abs_score = self.z_score.abs();
        if abs_score >= thresholds.critical {
            "critical"
        } else if abs_score >= thresholds.high {
            "high"
        } else if abs_score >= thresholds.medium {
            "medium"
        } else {
            "low"
        }
    }
}

/// Severity thresholds based on z-score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityThresholds {
    /// Z-score threshold for critical severity
    pub critical: f64,
    /// Z-score threshold for high severity
    pub high: f64,
    /// Z-score threshold for medium severity
    pub medium: f64,
}

impl Default for SeverityThresholds {
    fn default() -> Self {
        Self {
            critical: 4.0,
            high: 3.0,
            medium: 2.0,
        }
    }
}

/// Escalation policy for a severity level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationPolicy {
    /// Severity level this policy applies to
    pub severity_level: String,
    /// Whether to auto-escalate
    pub auto_escalate: bool,
    /// Notification channels
    pub notify_channels: Vec<String>,
    /// Response SLA in minutes
    pub response_sla_minutes: i64,
}

/// Output for anomaly triage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnomalyTriageOutput {
    /// Triaged anomalies
    pub triaged: Vec<TriagedAnomaly>,
    /// Count of anomalies requiring escalation
    pub escalation_count: usize,
    /// Summary by severity
    pub severity_summary: SeveritySummary,
}

impl AnomalyTriageOutput {
    /// Create empty output
    pub fn empty() -> Self {
        Self::default()
    }

    /// Generate a summary string
    pub fn summary(&self) -> String {
        format!(
            "Triaged {} anomalies: {} critical, {} high, {} medium, {} low ({} escalated)",
            self.triaged.len(),
            self.severity_summary.critical,
            self.severity_summary.high,
            self.severity_summary.medium,
            self.severity_summary.low,
            self.escalation_count
        )
    }
}

/// A triaged anomaly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriagedAnomaly {
    /// Anomaly identifier
    pub anomaly_id: String,
    /// Assigned severity
    pub severity: String,
    /// Priority rank (1 = highest)
    pub priority: usize,
    /// Whether escalation is required
    pub escalate: bool,
    /// Reason for classification
    pub reason: String,
    /// Recommended actions
    pub recommended_actions: Vec<String>,
}

/// Summary of anomalies by severity
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeveritySummary {
    /// Count of critical anomalies
    pub critical: usize,
    /// Count of high anomalies
    pub high: usize,
    /// Count of medium anomalies
    pub medium: usize,
    /// Count of low anomalies
    pub low: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_anomaly(id: &str, z_score: f64) -> Anomaly {
        Anomaly {
            id: id.to_string(),
            timestamp: 1700000000,
            source: "test-system".to_string(),
            z_score,
            features: serde_json::json!({}),
        }
    }

    #[test]
    fn test_severity_classification() {
        let thresholds = SeverityThresholds::default();

        let critical = create_test_anomaly("a1", 5.0);
        assert_eq!(critical.classify_severity(&thresholds), "critical");

        let high = create_test_anomaly("a2", 3.5);
        assert_eq!(high.classify_severity(&thresholds), "high");

        let medium = create_test_anomaly("a3", 2.5);
        assert_eq!(medium.classify_severity(&thresholds), "medium");

        let low = create_test_anomaly("a4", 1.0);
        assert_eq!(low.classify_severity(&thresholds), "low");
    }

    #[test]
    fn test_negative_z_score() {
        let thresholds = SeverityThresholds::default();
        let anomaly = create_test_anomaly("a1", -4.5);
        assert_eq!(anomaly.classify_severity(&thresholds), "critical");
    }

    #[test]
    fn test_threshold_validation() {
        let mut input = AnomalyTriageInput {
            anomalies: vec![],
            thresholds: SeverityThresholds::default(),
            escalation_policies: vec![],
        };

        assert!(input.validate().is_ok());

        input.thresholds.high = 5.0; // Higher than critical
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_rejects_non_finite_threshold_and_z_score() {
        let mut input = AnomalyTriageInput {
            anomalies: vec![create_test_anomaly("a1", 1.0)],
            thresholds: SeverityThresholds {
                critical: f64::NAN,
                high: 3.0,
                medium: 2.0,
            },
            escalation_policies: vec![],
        };
        assert!(input.validate().is_err());

        input.thresholds = SeverityThresholds::default();
        input.anomalies[0].z_score = f64::INFINITY;
        assert!(input.validate().is_err());
    }
}
