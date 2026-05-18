//! Types for Vendor Shortlist pack

use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

/// Input for vendor shortlist optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorShortlistInput {
    /// Vendors to evaluate
    pub vendors: Vec<Vendor>,
    /// Shortlist requirements
    pub requirements: ShortlistRequirements,
}

impl VendorShortlistInput {
    /// Validate the input
    pub fn validate(&self) -> Result<()> {
        if self.vendors.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one vendor is required",
            ));
        }
        if self.requirements.max_vendors == 0 {
            return Err(converge_pack::GateError::invalid_input(
                "max_vendors must be positive",
            ));
        }
        if !self.requirements.min_score.is_finite()
            || !(0.0..=100.0).contains(&self.requirements.min_score)
        {
            return Err(converge_pack::GateError::invalid_input(
                "min_score must be finite and between 0 and 100",
            ));
        }
        if !self.requirements.max_risk_score.is_finite()
            || !(0.0..=100.0).contains(&self.requirements.max_risk_score)
        {
            return Err(converge_pack::GateError::invalid_input(
                "max_risk_score must be finite and between 0 and 100",
            ));
        }
        for vendor in &self.vendors {
            if !vendor.score.is_finite() || !(0.0..=100.0).contains(&vendor.score) {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Vendor {} has invalid score: must be finite and between 0 and 100",
                    vendor.id
                )));
            }
            if !vendor.risk_score.is_finite() || !(0.0..=100.0).contains(&vendor.risk_score) {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Vendor {} has invalid risk_score: must be finite and between 0 and 100",
                    vendor.id
                )));
            }
        }
        Ok(())
    }

    /// Get vendors meeting minimum score
    pub fn vendors_meeting_min_score(&self) -> impl Iterator<Item = &Vendor> {
        let min = self.requirements.min_score;
        self.vendors.iter().filter(move |v| v.score >= min)
    }

    /// Get vendors within risk tolerance
    pub fn vendors_within_risk(&self) -> impl Iterator<Item = &Vendor> {
        let max = self.requirements.max_risk_score;
        self.vendors.iter().filter(move |v| v.risk_score <= max)
    }
}

/// A vendor to evaluate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vendor {
    /// Vendor identifier
    pub id: String,
    /// Vendor name
    pub name: String,
    /// Overall vendor score (0-100)
    pub score: f64,
    /// Risk score (0-100, lower is better)
    pub risk_score: f64,
    /// Compliance status
    pub compliance_status: String,
    /// List of certifications
    pub certifications: Vec<String>,
}

impl Vendor {
    /// Check if vendor has required certifications
    pub fn has_certifications(&self, required: &[String]) -> bool {
        required.iter().all(|req| self.certifications.contains(req))
    }

    /// Check if vendor is compliant
    pub fn is_compliant(&self) -> bool {
        self.compliance_status == "compliant" || self.compliance_status == "approved"
    }

    /// Calculate composite score (higher is better)
    pub fn composite_score(&self) -> f64 {
        // Score - (risk * 0.5) gives weighted composite
        self.score - (self.risk_score * 0.5)
    }
}

/// Shortlist requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortlistRequirements {
    /// Maximum vendors to include in shortlist
    pub max_vendors: usize,
    /// Minimum acceptable score
    pub min_score: f64,
    /// Maximum acceptable risk score
    pub max_risk_score: f64,
    /// Required certifications
    pub required_certifications: Vec<String>,
}

impl Default for ShortlistRequirements {
    fn default() -> Self {
        Self {
            max_vendors: 3,
            min_score: 0.0,
            max_risk_score: 100.0,
            required_certifications: vec![],
        }
    }
}

/// Output for vendor shortlist optimization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VendorShortlistOutput {
    /// Shortlisted vendors
    pub shortlist: Vec<ShortlistedVendor>,
    /// Rejected vendors with reasons
    pub rejected: Vec<RejectedVendor>,
    /// Summary statistics
    pub stats: ShortlistStats,
}

impl VendorShortlistOutput {
    /// Create empty shortlist
    pub fn empty(reason: &str) -> Self {
        Self {
            shortlist: vec![],
            rejected: vec![],
            stats: ShortlistStats {
                total_evaluated: 0,
                total_shortlisted: 0,
                total_rejected: 0,
                average_score: 0.0,
                reason: reason.to_string(),
            },
        }
    }

    /// Generate a summary string
    pub fn summary(&self) -> String {
        format!(
            "Shortlisted {} of {} vendors evaluated",
            self.stats.total_shortlisted, self.stats.total_evaluated
        )
    }
}

/// A shortlisted vendor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortlistedVendor {
    /// Vendor identifier
    pub vendor_id: String,
    /// Vendor name
    pub vendor_name: String,
    /// Rank in shortlist (1 = best)
    pub rank: usize,
    /// Vendor score
    pub score: f64,
    /// Composite score used for ranking
    pub composite_score: f64,
}

/// A rejected vendor with reason
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedVendor {
    /// Vendor identifier
    pub vendor_id: String,
    /// Vendor name
    pub vendor_name: String,
    /// Reason for rejection
    pub reason: String,
}

/// Shortlist statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShortlistStats {
    /// Total vendors evaluated
    pub total_evaluated: usize,
    /// Total vendors shortlisted
    pub total_shortlisted: usize,
    /// Total vendors rejected
    pub total_rejected: usize,
    /// Average score of shortlisted vendors
    pub average_score: f64,
    /// Additional notes/reason
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_vendor(id: &str, score: f64, risk: f64) -> Vendor {
        Vendor {
            id: id.to_string(),
            name: format!("Vendor {}", id),
            score,
            risk_score: risk,
            compliance_status: "compliant".to_string(),
            certifications: vec!["ISO9001".to_string()],
        }
    }

    #[test]
    fn test_vendor_composite_score() {
        let vendor = create_test_vendor("v1", 80.0, 20.0);
        // 80 - (20 * 0.5) = 70
        assert!((vendor.composite_score() - 70.0).abs() < 0.01);
    }

    #[test]
    fn test_has_certifications() {
        let vendor = create_test_vendor("v1", 80.0, 20.0);

        assert!(vendor.has_certifications(&["ISO9001".to_string()]));
        assert!(!vendor.has_certifications(&["SOC2".to_string()]));
    }

    #[test]
    fn test_input_validation() {
        let mut input = VendorShortlistInput {
            vendors: vec![create_test_vendor("v1", 80.0, 20.0)],
            requirements: ShortlistRequirements {
                max_vendors: 3,
                min_score: 50.0,
                max_risk_score: 50.0,
                required_certifications: vec![],
            },
        };

        assert!(input.validate().is_ok());

        input.requirements.max_vendors = 0;
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_rejects_non_finite_or_out_of_range_scores() {
        let mut input = VendorShortlistInput {
            vendors: vec![create_test_vendor("v1", f64::NAN, 20.0)],
            requirements: ShortlistRequirements::default(),
        };
        assert!(input.validate().is_err());

        input.vendors[0].score = 80.0;
        input.vendors[0].risk_score = 101.0;
        assert!(input.validate().is_err());

        input.vendors[0].risk_score = 20.0;
        input.requirements.min_score = f64::INFINITY;
        assert!(input.validate().is_err());

        input.requirements.min_score = 50.0;
        input.requirements.max_risk_score = -1.0;
        assert!(input.validate().is_err());
    }
}
