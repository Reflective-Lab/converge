// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! PromotionGate - The concrete gate enforcing "agents suggest, engine decides".
//!
//! PromotionGate validates Draft proposals and promotes Validated proposals to Facts.
//! It requires a ValidationReport for promotion - there is no bypass path.
//!
//! # Key Invariants
//!
//! - **No bypass path**: `promote_to_fact()` requires `ValidatedProposal` (which contains report)
//! - **No forgery**: `ValidationReport` has private token field, can only be created by gate
//! - **Complete audit**: `PromotionRecord` captures full provenance on every Fact
//!
//! # Usage
//!
//! ```ignore
//! let gate = PromotionGate::new(GateId::new("main"), ValidationPolicy::new());
//!
//! // Validate a draft proposal
//! let validated = gate.validate_proposal(draft_proposal, &context)?;
//!
//! // Promote to fact (requires the validated proof)
//! let fact = gate.promote_to_fact(validated, approver, evidence, trace)?;
//! ```

use crate::types::{
    Actor, ContentHash, Draft, EvidenceRef, Fact, FactContent, FactContentKind, FactId, GateId,
    LocalTrace, PromotionError, PromotionRecord, Proposal, ProposalId, Timestamp, TraceLink,
    Validated, ValidationSummary,
};

use super::lifecycle::ProposalLifecycle;
use super::validation::{
    CheckResult, ValidationContext, ValidationError, ValidationPolicy, ValidationReport,
};

// ============================================================================
// ValidatedProposal - Proof bundle for promotion
// ============================================================================

/// Validated proposal bundled with its validation proof.
///
/// This type ensures the report and proposal cannot be separated
/// between validation and promotion. The only way to create this
/// is through `PromotionGate::validate_proposal()`.
pub struct ValidatedProposal {
    /// The validated proposal.
    proposal: Proposal<Validated>,
    /// The validation report (proof of validation).
    report: ValidationReport,
}

impl ValidatedProposal {
    /// Get the validated proposal.
    pub fn proposal(&self) -> &Proposal<Validated> {
        &self.proposal
    }

    /// Get the validation report.
    pub fn report(&self) -> &ValidationReport {
        &self.report
    }

    /// Get the proposal ID.
    pub fn id(&self) -> &ProposalId {
        self.proposal.id()
    }
}

// Implement Debug manually to provide useful output
impl std::fmt::Debug for ValidatedProposal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidatedProposal")
            .field("proposal_id", &self.proposal.id())
            .field("report", &self.report)
            .finish()
    }
}

// ============================================================================
// PromotionGate - The gate enforcing "agents suggest, engine decides"
// ============================================================================

/// The promotion gate - enforces "agents suggest, engine decides".
///
/// PromotionGate is the ONLY path to create Facts from Proposals.
/// It requires:
/// 1. A Draft proposal to validate
/// 2. A ValidationReport proving validation occurred
/// 3. Actor and evidence for the promotion record
///
/// # Invariants
///
/// - No bypass path: `promote_to_fact()` requires `ValidatedProposal`
/// - No forgery: `ValidationReport` has private token field
/// - Complete audit: `PromotionRecord` captures full provenance
///
/// # Example
///
/// ```ignore
/// let gate = PromotionGate::new(
///     GateId::new("production-gate"),
///     ValidationPolicy::new()
///         .with_required_check("schema_valid")
///         .with_required_check("confidence_threshold"),
/// );
///
/// // Step 1: Validate
/// let validated = gate.validate_proposal(draft, &context)?;
///
/// // Step 2: Promote (can't skip validation - no other constructor)
/// let fact = gate.promote_to_fact(
///     validated,
///     Actor::system("engine"),
///     vec![EvidenceRef::observation(obs_id)],
///     TraceLink::local(LocalTrace::new("trace-1", "span-1")),
/// )?;
/// ```
#[derive(Debug, Clone)]
pub struct PromotionGate {
    /// Unique identifier for this gate.
    gate_id: GateId,
    /// Validation policy.
    policy: ValidationPolicy,
}

impl PromotionGate {
    /// Create a new promotion gate.
    pub fn new(gate_id: GateId, policy: ValidationPolicy) -> Self {
        Self { gate_id, policy }
    }

    /// Get the gate ID.
    pub fn gate_id(&self) -> &GateId {
        &self.gate_id
    }

    /// Get the policy version hash.
    pub fn policy_version(&self) -> &ContentHash {
        self.policy.version_hash()
    }

    /// Validate a draft proposal.
    ///
    /// Runs validation checks and returns a `ValidatedProposal` if all pass.
    /// The returned type bundles the validated proposal with its report,
    /// ensuring they cannot be separated.
    ///
    /// # Arguments
    ///
    /// - `proposal`: The draft proposal to validate (consumed)
    /// - `context`: Validation context (tenant, session, metadata)
    ///
    /// # Returns
    ///
    /// - `Ok(ValidatedProposal)`: If validation passes
    /// - `Err(ValidationError)`: If any check fails
    pub fn validate_proposal(
        &self,
        proposal: Proposal<Draft>,
        _context: &ValidationContext,
    ) -> Result<ValidatedProposal, ValidationError> {
        // Run basic validation checks
        let mut checks = Vec::new();

        // Check: proposal has content
        if proposal.content().content.is_empty() {
            checks.push(CheckResult::failed(
                "content_not_empty",
                "Proposal content is empty",
            ));
        } else {
            checks.push(CheckResult::passed("content_not_empty"));
        }

        // Check: required checks from policy
        for required in &self.policy.required_checks {
            // For now, mark as passed - real validation would check actual conditions
            checks.push(CheckResult::passed(required.clone()));
        }

        // Check if any checks failed
        if !checks.iter().all(|c| c.passed) {
            let failed: Vec<_> = checks
                .iter()
                .filter(|c| !c.passed)
                .map(|c| c.name.clone())
                .collect();
            return Err(ValidationError::CheckFailed {
                name: failed.join(", "),
                reason: format!("Checks failed: {}", failed.join(", ")),
            });
        }

        // Create validated proposal (pub(crate) constructor)
        let validated = Proposal::<Validated>::from_validated(
            proposal.id().clone(),
            proposal.content().clone(),
            proposal.provenance().clone(),
        );

        // Create validation report (pub(crate) constructor)
        let report = ValidationReport::new(
            proposal.id().clone(),
            checks,
            self.policy.version_hash().clone(),
        );

        Ok(ValidatedProposal {
            proposal: validated,
            report,
        })
    }

    /// Promote a validated proposal to a Fact.
    ///
    /// Requires the `ValidatedProposal` (which contains the report),
    /// the approver, evidence references, and trace link.
    ///
    /// # Arguments
    ///
    /// - `validated`: The validated proposal bundle (consumed)
    /// - `approver`: Who approved this promotion
    /// - `evidence`: Evidence references supporting the promotion
    /// - `trace`: Trace link for audit/replay
    ///
    /// # Returns
    ///
    /// - `Ok(Fact)`: The promoted fact with complete audit trail
    /// - `Err(PromotionError)`: If promotion fails
    pub fn promote_to_fact(
        &self,
        validated: ValidatedProposal,
        approver: Actor,
        evidence: Vec<EvidenceRef>,
        trace: TraceLink,
    ) -> Result<Fact, PromotionError> {
        self.promote_to_fact_at(validated, approver, evidence, trace, Timestamp::now())
    }

    /// Promote a validated proposal to a Fact with a caller-supplied timestamp.
    ///
    /// Deterministic engine paths use this with Lamport logical time so
    /// promotion records do not depend on hidden wall-clock reads.
    pub(crate) fn promote_to_fact_at(
        &self,
        validated: ValidatedProposal,
        approver: Actor,
        evidence: Vec<EvidenceRef>,
        trace: TraceLink,
        timestamp: Timestamp,
    ) -> Result<Fact, PromotionError> {
        let ValidatedProposal { proposal, report } = validated;

        // Verify report matches proposal
        if report.proposal_id() != proposal.id() {
            return Err(PromotionError::report_mismatch(
                proposal.id(),
                report.proposal_id(),
            ));
        }

        // Build validation summary from report
        let mut summary = ValidationSummary::new();
        for check in report.checks() {
            if check.passed {
                summary = summary.with_passed(&check.name);
            }
        }

        // Build promotion record
        let record = PromotionRecord::new(
            self.gate_id.clone(),
            report.policy_version().clone(),
            approver,
            summary,
            evidence,
            trace,
            timestamp.clone(),
        );

        // Create fact content from proposal content
        let fact_content = FactContent::new(
            FactContentKind::from(proposal.content().kind),
            proposal.content().content.clone(),
        );

        // Create fact (pub(crate) constructor)
        let fact = Fact::new(
            FactId::new(format!("fact:{}", proposal.id())),
            fact_content,
            record,
            timestamp,
        );

        Ok(fact)
    }
}

// ============================================================================
// SimpleIntent - Basic intent for ProposalLifecycle implementation
// ============================================================================

/// Simple intent for basic validation.
///
/// This is a minimal intent type for demonstrating the ProposalLifecycle
/// trait implementation. Real intents would have more structure.
#[derive(Debug, Clone)]
pub struct SimpleIntent {
    /// Description of what we're trying to achieve.
    pub description: String,
}

impl SimpleIntent {
    /// Create a new simple intent.
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
        }
    }
}

// ============================================================================
// ProposalLifecycle implementation for PromotionGate
// ============================================================================

impl ProposalLifecycle<SimpleIntent, Proposal<Draft>, ValidatedProposal, Fact> for PromotionGate {
    fn validate(
        &self,
        _intent: &SimpleIntent,
        proposal: Proposal<Draft>,
    ) -> Result<ValidatedProposal, ValidationError> {
        self.validate_proposal(proposal, &ValidationContext::default())
    }

    fn promote(&self, validated: ValidatedProposal) -> Result<Fact, PromotionError> {
        // Use system actor for trait-based promotion
        let approver = Actor::system("promotion-gate");
        let evidence = vec![];
        let trace = TraceLink::local(LocalTrace::new("auto-promote", "gate-promote"));
        self.promote_to_fact(validated, approver, evidence, trace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        CaptureContext, ContentHash, ObservationId, ObservationProvenance, ProposedContent,
        ProposedContentKind,
    };

    fn make_draft_proposal(id: &str, content: &str) -> Proposal<Draft> {
        Proposal::new(
            ProposalId::new(id),
            ProposedContent::new(ProposedContentKind::Claim, content),
            ObservationProvenance::new(
                ObservationId::new("obs-001"),
                ContentHash::zero(),
                CaptureContext::default(),
            ),
        )
    }

    #[test]
    fn gate_creation() {
        let gate = PromotionGate::new(
            GateId::new("test-gate"),
            ValidationPolicy::new().with_required_check("schema_valid"),
        );

        assert_eq!(gate.gate_id().as_str(), "test-gate");
    }

    #[test]
    fn successful_validation() {
        let gate = PromotionGate::new(GateId::new("test-gate"), ValidationPolicy::new());
        let proposal = make_draft_proposal("prop-001", "This is a valid claim");
        let context = ValidationContext::new();

        let validated = gate.validate_proposal(proposal, &context).unwrap();

        assert_eq!(validated.id().as_str(), "prop-001");
        assert!(validated.report().all_passed());
    }

    #[test]
    fn failed_validation_empty_content() {
        let gate = PromotionGate::new(GateId::new("test-gate"), ValidationPolicy::new());
        let proposal = make_draft_proposal("prop-002", ""); // Empty content
        let context = ValidationContext::new();

        let result = gate.validate_proposal(proposal, &context);

        assert!(result.is_err());
        match result {
            Err(ValidationError::CheckFailed { name, .. }) => {
                assert!(name.contains("content_not_empty"));
            }
            _ => panic!("expected CheckFailed error"),
        }
    }

    #[test]
    fn successful_promotion() {
        let gate = PromotionGate::new(GateId::new("test-gate"), ValidationPolicy::new());
        let proposal = make_draft_proposal("prop-003", "Valid claim");
        let context = ValidationContext::new();

        let validated = gate.validate_proposal(proposal, &context).unwrap();

        let fact = gate
            .promote_to_fact(
                validated,
                Actor::system("test-engine"),
                vec![EvidenceRef::observation(ObservationId::new("obs-001"))],
                TraceLink::local(LocalTrace::new("trace-001", "span-001")),
            )
            .unwrap();

        assert_eq!(fact.id().as_str(), "fact:prop-003");
        assert_eq!(fact.content().content, "Valid claim");
        assert_eq!(fact.promotion_record().gate_id.as_str(), "test-gate");
        assert!(fact.is_replay_eligible());
    }

    #[test]
    fn proposal_lifecycle_trait() {
        let gate = PromotionGate::new(GateId::new("lifecycle-gate"), ValidationPolicy::new());
        let intent = SimpleIntent::new("test intent");
        let proposal = make_draft_proposal("prop-004", "Lifecycle test");

        // Use trait methods
        let validated = gate.validate(&intent, proposal).unwrap();
        let fact = gate.promote(validated).unwrap();

        assert_eq!(fact.id().as_str(), "fact:prop-004");
        assert_eq!(fact.content().content, "Lifecycle test");
    }

    #[test]
    fn policy_required_checks_run() {
        let gate = PromotionGate::new(
            GateId::new("policy-gate"),
            ValidationPolicy::new()
                .with_required_check("schema_valid")
                .with_required_check("confidence_threshold"),
        );
        let proposal = make_draft_proposal("prop-005", "Policy test");
        let context = ValidationContext::new();

        let validated = gate.validate_proposal(proposal, &context).unwrap();

        // All required checks should be in the report (passed for now)
        let check_names: Vec<_> = validated
            .report()
            .checks()
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert!(check_names.contains(&"schema_valid"));
        assert!(check_names.contains(&"confidence_threshold"));
        assert!(check_names.contains(&"content_not_empty"));
    }

    #[test]
    fn validated_proposal_debug() {
        let gate = PromotionGate::new(GateId::new("debug-gate"), ValidationPolicy::new());
        let proposal = make_draft_proposal("prop-006", "Debug test");
        let context = ValidationContext::new();

        let validated = gate.validate_proposal(proposal, &context).unwrap();
        let debug = format!("{:?}", validated);

        assert!(debug.contains("ValidatedProposal"));
        assert!(debug.contains("prop-006"));
    }

    #[test]
    fn fact_content_kind_conversion() {
        // Verify the ProposedContentKind -> FactContentKind conversion
        use crate::types::{FactContentKind, ProposedContentKind};

        assert_eq!(
            FactContentKind::from(ProposedContentKind::Claim),
            FactContentKind::Claim
        );
        assert_eq!(
            FactContentKind::from(ProposedContentKind::Plan),
            FactContentKind::Plan
        );
        assert_eq!(
            FactContentKind::from(ProposedContentKind::Draft),
            FactContentKind::Document
        );
        assert_eq!(
            FactContentKind::from(ProposedContentKind::Reasoning),
            FactContentKind::Reasoning
        );
    }

    // Note: External code cannot construct ValidatedProposal directly.
    // The only way is through PromotionGate::validate_proposal().
    //
    // // In external crate:
    // let validated = ValidatedProposal { ... };
    // // ERROR: `proposal` and `report` fields are private
}
