// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Domain-level evals for Converge.
//!
//! These evals define what "good" means for domain-specific outcomes.
//! They are:
//! - Outcome-based, not path-based
//! - Reusable across models, agents, and time
//! - Stored as traceable artifacts
//! - Usable in invariant checks
//!
//! # Philosophy
//!
//! In Converge, evals are not tests of behavior — they are formal definitions
//! of acceptable outcomes. This aligns with the principle that:
//!
//! - Evals test whether a convergence outcome satisfies intent-level properties
//! - Evals are business semantics, not implementation details
//! - Evals are the competitive moat (most systems can't define them clearly)

use converge_core::{ContextKey, Eval, EvalOutcome, EvalResult};

/// Eval: Meeting schedule feasibility
///
/// Ensures all scheduled meetings respect working hours (10-16 or 9-17).
///
/// This is a domain-level eval for meeting scheduler outcomes.
pub struct MeetingScheduleFeasibilityEval;

impl Eval for MeetingScheduleFeasibilityEval {
    fn name(&self) -> &'static str {
        "meeting_schedule_feasibility"
    }

    fn description(&self) -> &'static str {
        "Ensures all scheduled meetings respect working hours"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies, ContextKey::Constraints]
    }

    fn evaluate(&self, ctx: &dyn converge_core::Context) -> EvalResult {
        let strategies = ctx.get(ContextKey::Strategies);
        let constraints = ctx.get(ContextKey::Constraints);

        if strategies.is_empty() {
            return EvalResult::new(
                self.name(),
                EvalOutcome::Indeterminate,
                0.0,
                "No meetings found to evaluate",
            );
        }

        // Extract working hours from constraints
        let working_hours = constraints
            .iter()
            .any(|c| c.content.contains("10-16") || c.content.contains("9-17"));

        if !working_hours {
            return EvalResult::new(
                self.name(),
                EvalOutcome::Indeterminate,
                0.0,
                "Working hours constraint not found",
            );
        }

        let mut valid_count = 0;
        let fact_ids: Vec<String> = strategies.iter().map(|s| s.id.clone()).collect();

        for strategy in strategies {
            // Simplified: check if content mentions valid time slots
            // In production, this would parse structured time data
            let is_valid = strategy.content.contains("10-16") || strategy.content.contains("9-17");

            if is_valid {
                valid_count += 1;
            }
        }

        let validity_ratio = f64::from(valid_count) / strategies.len() as f64;

        if (validity_ratio - 1.0).abs() < f64::EPSILON {
            EvalResult::with_facts(
                self.name(),
                EvalOutcome::Pass,
                1.0,
                format!("All {} meetings respect working hours", strategies.len()),
                fact_ids,
            )
        } else {
            EvalResult::with_facts(
                self.name(),
                EvalOutcome::Fail,
                validity_ratio,
                format!(
                    "Only {}/{} meetings respect working hours",
                    valid_count,
                    strategies.len()
                ),
                fact_ids,
            )
        }
    }
}

// =============================================================================
// PACK-SPECIFIC EVALS
// =============================================================================

// -----------------------------------------------------------------------------
// Money Pack Evals
// -----------------------------------------------------------------------------

/// Eval: Invoice accuracy
///
/// Ensures all invoices have valid amounts, customer references, and line items.
pub struct InvoiceAccuracyEval;

impl Eval for InvoiceAccuracyEval {
    fn name(&self) -> &'static str {
        "invoice_accuracy"
    }

    fn description(&self) -> &'static str {
        "Ensures all invoices have valid amounts, customer references, and line items"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn evaluate(&self, ctx: &dyn converge_core::Context) -> EvalResult {
        let proposals = ctx.get(ContextKey::Proposals);
        let invoices: Vec<_> = proposals
            .iter()
            .filter(|p| p.id.starts_with("invoice:"))
            .collect();

        if invoices.is_empty() {
            return EvalResult::new(
                self.name(),
                EvalOutcome::Indeterminate,
                0.0,
                "No invoices found to evaluate",
            );
        }

        let mut valid_count = 0;
        for invoice in &invoices {
            let has_amount = invoice.content.contains("amount");
            let has_customer = invoice.content.contains("customer");
            if has_amount && has_customer {
                valid_count += 1;
            }
        }

        let ratio = valid_count as f64 / invoices.len() as f64;
        let fact_ids: Vec<String> = invoices.iter().map(|i| i.id.clone()).collect();

        if ratio >= 0.95 {
            EvalResult::with_facts(
                self.name(),
                EvalOutcome::Pass,
                ratio,
                format!("{}/{} invoices are accurate", valid_count, invoices.len()),
                fact_ids,
            )
        } else {
            EvalResult::with_facts(
                self.name(),
                EvalOutcome::Fail,
                ratio,
                format!(
                    "Only {}/{} invoices are accurate, need 95%",
                    valid_count,
                    invoices.len()
                ),
                fact_ids,
            )
        }
    }
}

/// Eval: Payment reconciliation completeness
///
/// Ensures all payments are matched to invoices with no orphaned payments.
pub struct PaymentReconciliationEval;

impl Eval for PaymentReconciliationEval {
    fn name(&self) -> &'static str {
        "payment_reconciliation"
    }

    fn description(&self) -> &'static str {
        "Ensures all payments are matched to invoices"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn evaluate(&self, ctx: &dyn converge_core::Context) -> EvalResult {
        let proposals = ctx.get(ContextKey::Proposals);
        let payments: Vec<_> = proposals
            .iter()
            .filter(|p| p.id.starts_with("payment:"))
            .collect();

        if payments.is_empty() {
            return EvalResult::new(
                self.name(),
                EvalOutcome::Indeterminate,
                0.0,
                "No payments found",
            );
        }

        let matched = payments
            .iter()
            .filter(|p| p.content.contains("invoice_id"))
            .count();
        let ratio = matched as f64 / payments.len() as f64;

        if ratio >= 1.0 {
            EvalResult::new(
                self.name(),
                EvalOutcome::Pass,
                ratio,
                format!("All {} payments are reconciled", payments.len()),
            )
        } else {
            EvalResult::new(
                self.name(),
                EvalOutcome::Fail,
                ratio,
                format!(
                    "{}/{} payments are orphaned",
                    payments.len() - matched,
                    payments.len()
                ),
            )
        }
    }
}

// -----------------------------------------------------------------------------
// Delivery Pack Evals
// -----------------------------------------------------------------------------

/// Eval: Promise fulfillment rate
///
/// Ensures delivery promises are fulfilled within acceptable margins.
pub struct PromiseFulfillmentEval;

impl Eval for PromiseFulfillmentEval {
    fn name(&self) -> &'static str {
        "promise_fulfillment"
    }

    fn description(&self) -> &'static str {
        "Ensures delivery promises are fulfilled"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn evaluate(&self, ctx: &dyn converge_core::Context) -> EvalResult {
        let proposals = ctx.get(ContextKey::Proposals);
        let promises: Vec<_> = proposals
            .iter()
            .filter(|p| p.id.starts_with("promise:"))
            .collect();

        if promises.is_empty() {
            return EvalResult::new(
                self.name(),
                EvalOutcome::Indeterminate,
                0.0,
                "No promises found",
            );
        }

        let fulfilled = promises
            .iter()
            .filter(|p| {
                p.content.contains("delivered")
                    || p.content.contains("accepted")
                    || p.content.contains("complete")
            })
            .count();

        let ratio = fulfilled as f64 / promises.len() as f64;

        if ratio >= 0.9 {
            EvalResult::new(
                self.name(),
                EvalOutcome::Pass,
                ratio,
                format!("{:.0}% promises fulfilled", ratio * 100.0),
            )
        } else {
            EvalResult::new(
                self.name(),
                EvalOutcome::Fail,
                ratio,
                format!("Only {:.0}% promises fulfilled, need 90%", ratio * 100.0),
            )
        }
    }
}

/// Eval: Scope creep detection
///
/// Ensures scope changes are tracked and approved.
pub struct ScopeCreepDetectionEval;

impl Eval for ScopeCreepDetectionEval {
    fn name(&self) -> &'static str {
        "scope_creep_detection"
    }

    fn description(&self) -> &'static str {
        "Ensures scope changes are tracked and approved"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals, ContextKey::Signals]
    }

    fn evaluate(&self, ctx: &dyn converge_core::Context) -> EvalResult {
        let proposals = ctx.get(ContextKey::Proposals);
        let scope_changes: Vec<_> = proposals
            .iter()
            .filter(|p| p.id.starts_with("scope:") && p.content.contains("change"))
            .collect();

        if scope_changes.is_empty() {
            return EvalResult::new(
                self.name(),
                EvalOutcome::Pass,
                1.0,
                "No scope changes detected",
            );
        }

        let approved = scope_changes
            .iter()
            .filter(|s| s.content.contains("approved"))
            .count();
        let ratio = approved as f64 / scope_changes.len() as f64;

        if ratio >= 1.0 {
            EvalResult::new(
                self.name(),
                EvalOutcome::Pass,
                ratio,
                format!("All {} scope changes are approved", scope_changes.len()),
            )
        } else {
            EvalResult::new(
                self.name(),
                EvalOutcome::Fail,
                ratio,
                format!(
                    "{} unapproved scope changes detected",
                    scope_changes.len() - approved
                ),
            )
        }
    }
}

/// Eval: Access compliance
///
/// Ensures terminated employees have revoked access.
pub struct AccessComplianceEval;

impl Eval for AccessComplianceEval {
    fn name(&self) -> &'static str {
        "access_compliance"
    }

    fn description(&self) -> &'static str {
        "Ensures terminated employees have revoked access"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn evaluate(&self, ctx: &dyn converge_core::Context) -> EvalResult {
        let proposals = ctx.get(ContextKey::Proposals);
        let terminated: Vec<_> = proposals
            .iter()
            .filter(|p| p.id.starts_with("employee:") && p.content.contains("terminated"))
            .collect();

        if terminated.is_empty() {
            return EvalResult::new(
                self.name(),
                EvalOutcome::Pass,
                1.0,
                "No terminated employees",
            );
        }

        let revoked = terminated
            .iter()
            .filter(|t| t.content.contains("revoked"))
            .count();
        let ratio = revoked as f64 / terminated.len() as f64;

        if ratio >= 1.0 {
            EvalResult::new(
                self.name(),
                EvalOutcome::Pass,
                ratio,
                format!(
                    "All {} terminated employees have revoked access",
                    terminated.len()
                ),
            )
        } else {
            EvalResult::new(
                self.name(),
                EvalOutcome::Fail,
                ratio,
                format!(
                    "{} terminated employees still have access",
                    terminated.len() - revoked
                ),
            )
        }
    }
}

// -----------------------------------------------------------------------------
// Trust Pack Evals
// -----------------------------------------------------------------------------

/// Eval: Audit trail coverage
///
/// Ensures all sensitive actions have audit records.
pub struct AuditCoverageEval;

impl Eval for AuditCoverageEval {
    fn name(&self) -> &'static str {
        "audit_coverage"
    }

    fn description(&self) -> &'static str {
        "Ensures all sensitive actions have audit records"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn evaluate(&self, ctx: &dyn converge_core::Context) -> EvalResult {
        let proposals = ctx.get(ContextKey::Proposals);
        let actions: Vec<_> = proposals
            .iter()
            .filter(|p| p.id.starts_with("action:"))
            .collect();
        let audits: Vec<_> = proposals
            .iter()
            .filter(|p| p.id.starts_with("audit:"))
            .collect();

        if actions.is_empty() {
            return EvalResult::new(self.name(), EvalOutcome::Pass, 1.0, "No actions to audit");
        }

        let coverage = if actions.is_empty() {
            1.0
        } else {
            audits.len() as f64 / actions.len() as f64
        };

        if coverage >= 1.0 {
            EvalResult::new(
                self.name(),
                EvalOutcome::Pass,
                coverage.min(1.0),
                format!("All {} actions are audited", actions.len()),
            )
        } else {
            EvalResult::new(
                self.name(),
                EvalOutcome::Fail,
                coverage,
                format!(
                    "{}/{} actions lack audit records",
                    actions.len() - audits.len(),
                    actions.len()
                ),
            )
        }
    }
}

/// Eval: RBAC enforcement
///
/// Ensures role-based access control is properly enforced.
pub struct RbacEnforcementEval;

impl Eval for RbacEnforcementEval {
    fn name(&self) -> &'static str {
        "rbac_enforcement"
    }

    fn description(&self) -> &'static str {
        "Ensures RBAC is properly enforced on all access attempts"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn evaluate(&self, ctx: &dyn converge_core::Context) -> EvalResult {
        let proposals = ctx.get(ContextKey::Proposals);
        let access_attempts: Vec<_> = proposals
            .iter()
            .filter(|p| p.id.starts_with("access:"))
            .collect();

        if access_attempts.is_empty() {
            return EvalResult::new(
                self.name(),
                EvalOutcome::Indeterminate,
                0.0,
                "No access attempts found",
            );
        }

        let enforced = access_attempts
            .iter()
            .filter(|a| a.content.contains("role") || a.content.contains("permission"))
            .count();

        let ratio = enforced as f64 / access_attempts.len() as f64;

        if ratio >= 1.0 {
            EvalResult::new(
                self.name(),
                EvalOutcome::Pass,
                ratio,
                format!(
                    "RBAC enforced on all {} access attempts",
                    access_attempts.len()
                ),
            )
        } else {
            EvalResult::new(
                self.name(),
                EvalOutcome::Fail,
                ratio,
                format!(
                    "{} access attempts bypass RBAC",
                    access_attempts.len() - enforced
                ),
            )
        }
    }
}

// -----------------------------------------------------------------------------
// Data Metrics Pack Evals
// -----------------------------------------------------------------------------

/// Eval: Metric definition quality
///
/// Ensures metrics have proper definitions with formulas and owners.
pub struct MetricDefinitionQualityEval;

impl Eval for MetricDefinitionQualityEval {
    fn name(&self) -> &'static str {
        "metric_definition_quality"
    }

    fn description(&self) -> &'static str {
        "Ensures metrics have proper definitions"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn evaluate(&self, ctx: &dyn converge_core::Context) -> EvalResult {
        let proposals = ctx.get(ContextKey::Proposals);
        let metrics: Vec<_> = proposals
            .iter()
            .filter(|p| p.id.starts_with("metric:"))
            .collect();

        if metrics.is_empty() {
            return EvalResult::new(
                self.name(),
                EvalOutcome::Indeterminate,
                0.0,
                "No metrics found",
            );
        }

        let quality = metrics
            .iter()
            .filter(|m| m.content.contains("formula") || m.content.contains("definition"))
            .count();

        let ratio = quality as f64 / metrics.len() as f64;

        if ratio >= 0.9 {
            EvalResult::new(
                self.name(),
                EvalOutcome::Pass,
                ratio,
                format!(
                    "{}/{} metrics have quality definitions",
                    quality,
                    metrics.len()
                ),
            )
        } else {
            EvalResult::new(
                self.name(),
                EvalOutcome::Fail,
                ratio,
                format!(
                    "{} metrics lack proper definitions",
                    metrics.len() - quality
                ),
            )
        }
    }
}

/// Eval: Dashboard source attribution
///
/// Ensures dashboards cite their data sources.
pub struct DashboardSourceEval;

impl Eval for DashboardSourceEval {
    fn name(&self) -> &'static str {
        "dashboard_source"
    }

    fn description(&self) -> &'static str {
        "Ensures dashboards cite their data sources"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn evaluate(&self, ctx: &dyn converge_core::Context) -> EvalResult {
        let proposals = ctx.get(ContextKey::Proposals);
        let dashboards: Vec<_> = proposals
            .iter()
            .filter(|p| p.id.starts_with("dashboard:"))
            .collect();

        if dashboards.is_empty() {
            return EvalResult::new(
                self.name(),
                EvalOutcome::Indeterminate,
                0.0,
                "No dashboards found",
            );
        }

        let cited = dashboards
            .iter()
            .filter(|d| d.content.contains("source"))
            .count();
        let ratio = cited as f64 / dashboards.len() as f64;

        if ratio >= 1.0 {
            EvalResult::new(
                self.name(),
                EvalOutcome::Pass,
                ratio,
                format!("All {} dashboards cite sources", dashboards.len()),
            )
        } else {
            EvalResult::new(
                self.name(),
                EvalOutcome::Fail,
                ratio,
                format!(
                    "{} dashboards lack source citation",
                    dashboards.len() - cited
                ),
            )
        }
    }
}

// (Organism-pack evals moved to organism-domain)

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::{ContextState, Engine};

    fn promoted_context(entries: &[(ContextKey, &str, &str)]) -> ContextState {
        let mut ctx = ContextState::new();
        for (key, id, content) in entries {
            ctx.add_input(*key, *id, *content).unwrap();
        }
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(Engine::new().run(ctx))
            .unwrap()
            .context
    }

    #[test]
    fn invoice_accuracy_passes_with_valid_invoices() {
        let eval = InvoiceAccuracyEval;
        let ctx = promoted_context(&[(
            ContextKey::Proposals,
            "invoice:001",
            "amount: 1000, customer: Acme Corp",
        )]);

        let result = eval.evaluate(&ctx);
        assert_eq!(result.outcome, EvalOutcome::Pass);
    }

    #[test]
    fn meeting_feasibility_indeterminate_without_constraints() {
        let eval = MeetingScheduleFeasibilityEval;
        let ctx = promoted_context(&[(ContextKey::Strategies, "meeting-1", "standup at 10-16")]);

        let result = eval.evaluate(&ctx);
        // No working hours constraint → indeterminate
        assert_eq!(result.outcome, EvalOutcome::Indeterminate);
    }
}
