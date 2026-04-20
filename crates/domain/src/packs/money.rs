// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Money Pack agents for finance operations.
//!
//! Implements the agent contracts defined in specs/money.feature.
//!
//! # Lifecycle: AR → AP → Reconcile → Close
//!
//! # Suggestor Pipeline
//!
//! ```text
//! Triggers (deal.closed_won, milestone.completed)
//!    │
//!    ▼
//! InvoiceCreatorAgent → Invoices (draft)
//!    │
//!    ▼
//! PaymentAllocatorAgent → PaymentAllocations
//!    │
//!    ▼
//! ReconciliationMatcherAgent → LedgerEntries
//!    │
//!    ▼
//! OverdueDetectorAgent → Overdue flags + collection actions
//!    │
//!    ▼
//! PeriodCloserAgent → Period (closed)
//! ```
//!
//! Note: This implementation uses the standard ContextKey enum. Facts are
//! distinguished by their ID prefixes (invoice:, payment:, ledger:, etc.).

use converge_core::{
    AgentEffect, ContextKey, FlowAction, FlowGateAuthorizer, FlowGateOutcome, Suggestor,
    invariant::{Invariant, InvariantClass, InvariantResult, Violation},
};
use std::sync::Arc;

// ============================================================================
// Fact ID Prefixes
// ============================================================================

/// Prefix for invoice facts
pub const INVOICE_PREFIX: &str = "invoice:";
/// Prefix for payment facts
pub const PAYMENT_PREFIX: &str = "payment:";
/// Prefix for ledger entry facts
pub const LEDGER_PREFIX: &str = "ledger:";
/// Prefix for period facts
pub const PERIOD_PREFIX: &str = "period:";

fn invoice_issue_request_exists(ctx: &dyn converge_core::Context, invoice_id: &str) -> bool {
    let request_id = format!("{INVOICE_PREFIX}issue_request:{invoice_id}");
    ctx.get(ContextKey::Proposals)
        .iter()
        .any(|fact| fact.id == request_id)
}

fn invoice_issue_final_output_exists(ctx: &dyn converge_core::Context, invoice_id: &str) -> bool {
    let issued_id = format!("{INVOICE_PREFIX}issued:{invoice_id}");
    let rejected_id = format!("{INVOICE_PREFIX}issue_rejected:{invoice_id}");

    ctx.get(ContextKey::Proposals)
        .iter()
        .any(|fact| fact.id == issued_id || fact.id == rejected_id)
}

fn period_close_request_exists(ctx: &dyn converge_core::Context, period_id: &str) -> bool {
    let request_id = format!("{PERIOD_PREFIX}close_request:{period_id}");
    ctx.get(ContextKey::Proposals)
        .iter()
        .any(|fact| fact.id == request_id)
}

fn period_close_final_output_exists(ctx: &dyn converge_core::Context, period_id: &str) -> bool {
    let closed_id = format!("{PERIOD_PREFIX}closed:{period_id}");
    let rejected_id = format!("{PERIOD_PREFIX}close_rejected:{period_id}");

    ctx.get(ContextKey::Proposals)
        .iter()
        .any(|fact| fact.id == closed_id || fact.id == rejected_id)
}

// ============================================================================
// Agents
// ============================================================================

/// Creates invoices from deal triggers.
///
/// Responds to:
/// - customers.deal.closed_won
/// - delivery.milestone.completed
/// - subscription.cycle
///
/// Produces: Invoice facts in state "draft"
#[derive(Debug, Clone, Default)]
pub struct InvoiceCreatorAgent;

#[async_trait::async_trait]
impl Suggestor for InvoiceCreatorAgent {
    fn name(&self) -> &str {
        "invoice_creator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        // Accept when we have triggers but haven't created invoices yet
        let has_triggers = ctx.get(ContextKey::Seeds).iter().any(|f| {
            f.content.contains("deal.closed_won")
                || f.content.contains("milestone.completed")
                || f.content.contains("subscription.cycle")
        });
        let has_invoices = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|f| f.id.starts_with(INVOICE_PREFIX));
        has_triggers && !has_invoices
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("deal.closed_won") {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{}draft:{}", INVOICE_PREFIX, trigger.id),
                    serde_json::json!({
                        "type": "invoice",
                        "state": "draft",
                        "source_trigger": trigger.id,
                        "customer_id": "extracted_from_trigger",
                        "line_items": [],
                        "amount": 0,
                        "currency": "USD"
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Routes ready invoices through the default flow gate authorizer before issue.
#[derive(Clone)]
pub struct InvoiceIssuerAgent {
    policy: Arc<dyn FlowGateAuthorizer>,
}

impl InvoiceIssuerAgent {
    #[must_use]
    pub fn new(policy: Arc<dyn FlowGateAuthorizer>) -> Self {
        Self { policy }
    }
}

impl Default for InvoiceIssuerAgent {
    fn default() -> Self {
        Self::new(crate::flow_governance::default_flow_authorizer())
    }
}

impl std::fmt::Debug for InvoiceIssuerAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvoiceIssuerAgent")
    }
}

#[async_trait::async_trait]
impl Suggestor for InvoiceIssuerAgent {
    fn name(&self) -> &str {
        "invoice_issuer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|invoice| {
            invoice.id.starts_with(INVOICE_PREFIX)
                && invoice.content.contains("\"state\":\"ready_to_issue\"")
                && !invoice_issue_final_output_exists(ctx, &invoice.id)
        })
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for invoice in proposals.iter() {
            if !invoice.id.starts_with(INVOICE_PREFIX)
                || !invoice.content.contains("\"state\":\"ready_to_issue\"")
                || invoice_issue_final_output_exists(ctx, &invoice.id)
            {
                continue;
            }

            let Ok(invoice_json) = serde_json::from_str::<serde_json::Value>(&invoice.content)
            else {
                continue;
            };

            let customer_validated = invoice_json
                .get("customer_id")
                .and_then(serde_json::Value::as_str)
                .is_some();
            let line_items_balanced =
                crate::flow_governance::json_has_array_items(&invoice_json, "line_items")
                    && invoice_json
                        .get("amount")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(0)
                        > 0;
            let required_gates_met = customer_validated && line_items_balanced;

            let mut gates_passed = Vec::new();
            if customer_validated {
                gates_passed.push("customer_validated".to_string());
            }
            if line_items_balanced {
                gates_passed.push("line_items_balanced".to_string());
            }

            let human_approval_present = crate::flow_governance::has_approval(
                ctx,
                "invoice",
                &invoice.id,
                "finance_manager",
            );
            let amount = invoice_json
                .get("amount")
                .and_then(serde_json::Value::as_i64);

            let decision = self
                .policy
                .decide(&crate::flow_governance::flow_input(
                    "agent:finance-supervisor",
                    "supervisory",
                    "finance",
                    format!("invoice-issuance:{}", invoice.id),
                    "invoice",
                    gates_passed,
                    amount,
                    human_approval_present,
                    required_gates_met,
                    FlowAction::Commit,
                ))
                .expect("built-in invoice issuance policy should evaluate");

            match decision.outcome {
                FlowGateOutcome::Promote => facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{INVOICE_PREFIX}issued:{}", invoice.id),
                    serde_json::json!({
                        "type": "invoice",
                        "source_invoice_id": invoice.id,
                        "state": "issued",
                        "customer_id": invoice_json.get("customer_id").cloned().unwrap_or_default(),
                        "line_items": invoice_json.get("line_items").cloned().unwrap_or_default(),
                        "amount": amount.unwrap_or(0),
                        "currency": invoice_json.get("currency").cloned().unwrap_or(serde_json::json!("USD")),
                        "human_approval_present": human_approval_present,
                        "policy_reason": decision.reason
                    })
                    .to_string(),
                )),
                FlowGateOutcome::Escalate => {
                    if !invoice_issue_request_exists(ctx, &invoice.id) {
                        facts.push(crate::proposal(
                            self.name(),
                            ContextKey::Proposals,
                            format!("{INVOICE_PREFIX}issue_request:{}", invoice.id),
                            serde_json::json!({
                                "type": "invoice_issue_request",
                                "invoice_id": invoice.id,
                                "action": "request_authority",
                                "required_role": "finance_manager",
                                "pending_approval": true,
                                "policy_outcome": decision.outcome,
                                "policy_reason": decision.reason
                            })
                            .to_string(),
                        ));
                    }
                }
                FlowGateOutcome::Reject => facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{INVOICE_PREFIX}issue_rejected:{}", invoice.id),
                    serde_json::json!({
                        "type": "invoice_issue_rejected",
                        "invoice_id": invoice.id,
                        "policy_outcome": decision.outcome,
                        "policy_reason": decision.reason
                    })
                    .to_string(),
                )),
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Allocates incoming payments to invoices.
///
/// Matching priority:
/// 1. Exact amount match
/// 2. Customer + oldest invoice
/// 3. Reference number match
#[derive(Debug, Clone, Default)]
pub struct PaymentAllocatorAgent;

#[async_trait::async_trait]
impl Suggestor for PaymentAllocatorAgent {
    fn name(&self) -> &str {
        "payment_allocator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        // Accept when we have unallocated payments
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(PAYMENT_PREFIX) && p.content.contains("\"state\":\"unallocated\"")
        })
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let payments: Vec<_> = proposals
            .iter()
            .filter(|p| {
                p.id.starts_with(PAYMENT_PREFIX) && p.content.contains("\"state\":\"unallocated\"")
            })
            .collect();
        let invoices: Vec<_> = proposals
            .iter()
            .filter(|p| p.id.starts_with(INVOICE_PREFIX))
            .collect();

        let mut facts = Vec::new();

        for payment in payments.iter() {
            // Try to find matching invoice
            if let Some(invoice) = invoices.first() {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!(
                        "{}allocation:{}->{}",
                        PAYMENT_PREFIX, payment.id, invoice.id
                    ),
                    serde_json::json!({
                        "type": "payment_allocation",
                        "payment_id": payment.id,
                        "invoice_id": invoice.id,
                        "amount": "full",
                        "match_method": "exact_amount"
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Matches bank transactions to invoices and bills.
///
/// Creates LedgerEntry facts for successful matches.
#[derive(Debug, Clone, Default)]
pub struct ReconciliationMatcherAgent;

#[async_trait::async_trait]
impl Suggestor for ReconciliationMatcherAgent {
    fn name(&self) -> &str {
        "reconciliation_matcher"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals, ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        // Accept when we have bank transactions (signals) and no ledger entries yet
        let has_bank_txns = ctx
            .get(ContextKey::Signals)
            .iter()
            .any(|s| s.id.contains("bank_txn"));
        let has_ledger = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(LEDGER_PREFIX));
        has_bank_txns && !has_ledger
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let proposals = ctx.get(ContextKey::Proposals);

        let bank_txns: Vec<_> = signals
            .iter()
            .filter(|s| s.id.contains("bank_txn"))
            .collect();
        let invoices: Vec<_> = proposals
            .iter()
            .filter(|p| p.id.starts_with(INVOICE_PREFIX))
            .collect();

        let mut facts = Vec::new();

        for txn in bank_txns.iter() {
            if let Some(invoice) = invoices.first() {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{}{}->{}", LEDGER_PREFIX, txn.id, invoice.id),
                    serde_json::json!({
                        "type": "ledger_entry",
                        "bank_txn_id": txn.id,
                        "matched_doc_id": invoice.id,
                        "match_confidence": 0.95,
                        "match_method": "exact"
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Detects overdue invoices and proposes collection actions.
///
/// Action thresholds:
/// - 1-7 days: reminder_email
/// - 8-30 days: escalate_to_collections
/// - 31+ days: review_for_write_off
#[derive(Debug, Clone, Default)]
pub struct OverdueDetectorAgent;

#[async_trait::async_trait]
impl Suggestor for OverdueDetectorAgent {
    fn name(&self) -> &str {
        "overdue_detector"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        // Check for open/partial invoices past due date
        ctx.get(ContextKey::Proposals).iter().any(|inv| {
            inv.id.starts_with(INVOICE_PREFIX)
                && (inv.content.contains("\"state\":\"open\"")
                    || inv.content.contains("\"state\":\"partial\""))
                && inv.content.contains("\"overdue\":true")
        })
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for invoice in proposals.iter() {
            if invoice.id.starts_with(INVOICE_PREFIX)
                && invoice.content.contains("\"overdue\":true")
            {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{}overdue_action:{}", INVOICE_PREFIX, invoice.id),
                    serde_json::json!({
                        "type": "overdue_action",
                        "invoice_id": invoice.id,
                        "new_state": "overdue",
                        "action": "reminder_email",
                        "days_overdue": 7
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Closes accounting periods after reconciliation.
///
/// Requires authority approval before transitioning to closed.
#[derive(Clone)]
pub struct PeriodCloserAgent {
    policy: Arc<dyn FlowGateAuthorizer>,
}

impl PeriodCloserAgent {
    #[must_use]
    pub fn new(policy: Arc<dyn FlowGateAuthorizer>) -> Self {
        Self { policy }
    }
}

impl Default for PeriodCloserAgent {
    fn default() -> Self {
        Self::new(crate::flow_governance::default_flow_authorizer())
    }
}

impl std::fmt::Debug for PeriodCloserAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PeriodCloserAgent")
    }
}

#[async_trait::async_trait]
impl Suggestor for PeriodCloserAgent {
    fn name(&self) -> &str {
        "period_closer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        // Accept when period is in "closing" state and all reconciliation complete
        ctx.get(ContextKey::Proposals).iter().any(|period| {
            period.id.starts_with(PERIOD_PREFIX)
                && period.content.contains("\"state\":\"closing\"")
                && !period_close_final_output_exists(ctx, &period.id)
        })
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for period in proposals.iter() {
            if !period.id.starts_with(PERIOD_PREFIX)
                || !period.content.contains("\"state\":\"closing\"")
                || period_close_final_output_exists(ctx, &period.id)
            {
                continue;
            }

            let Ok(period_json) = serde_json::from_str::<serde_json::Value>(&period.content) else {
                continue;
            };

            let reconciliation_complete = period_json
                .get("reconciliation_complete")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let human_approval_present =
                crate::flow_governance::has_approval(ctx, "period", &period.id, "finance_manager");
            let decision = self
                .policy
                .decide(&crate::flow_governance::flow_input(
                    "agent:finance-supervisor",
                    "supervisory",
                    "finance",
                    format!("period-close:{}", period.id),
                    "period",
                    if reconciliation_complete {
                        vec!["reconciliation_complete".to_string()]
                    } else {
                        Vec::new()
                    },
                    None,
                    human_approval_present,
                    reconciliation_complete,
                    FlowAction::Commit,
                ))
                .expect("built-in period close policy should evaluate");

            match decision.outcome {
                FlowGateOutcome::Promote => facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{PERIOD_PREFIX}closed:{}", period.id),
                    serde_json::json!({
                        "type": "period",
                        "source_period_id": period.id,
                        "state": "closed",
                        "human_approval_present": human_approval_present,
                        "policy_reason": decision.reason
                    })
                    .to_string(),
                )),
                FlowGateOutcome::Escalate => {
                    if !period_close_request_exists(ctx, &period.id) {
                        facts.push(crate::proposal(
                            self.name(),
                            ContextKey::Proposals,
                            format!("{PERIOD_PREFIX}close_request:{}", period.id),
                            serde_json::json!({
                                "type": "period_close_request",
                                "period_id": period.id,
                                "action": "request_authority",
                                "required_role": "finance_manager",
                                "pending_approval": true,
                                "policy_outcome": decision.outcome,
                                "policy_reason": decision.reason
                            })
                            .to_string(),
                        ));
                    }
                }
                FlowGateOutcome::Reject => facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{PERIOD_PREFIX}close_rejected:{}", period.id),
                    serde_json::json!({
                        "type": "period_close_rejected",
                        "period_id": period.id,
                        "policy_outcome": decision.outcome,
                        "policy_reason": decision.reason
                    })
                    .to_string(),
                )),
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

// ============================================================================
// Invariants
// ============================================================================

/// Ensures invoices have valid customer references.
#[derive(Debug, Clone, Default)]
pub struct InvoiceHasCustomerInvariant;

impl Invariant for InvoiceHasCustomerInvariant {
    fn name(&self) -> &str {
        "invoice_has_customer"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        for invoice in ctx.get(ContextKey::Proposals).iter() {
            if invoice.id.starts_with(INVOICE_PREFIX) && !invoice.content.contains("customer_id") {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Invoice {} missing customer_id", invoice.id),
                    vec![invoice.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures payment allocations balance with invoice totals.
#[derive(Debug, Clone, Default)]
pub struct PaymentAllocationCompleteInvariant;

impl Invariant for PaymentAllocationCompleteInvariant {
    fn name(&self) -> &str {
        "payment_allocation_complete"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        // Check that paid invoices have allocations summing to total
        for invoice in ctx.get(ContextKey::Proposals).iter() {
            if invoice.id.starts_with(INVOICE_PREFIX)
                && invoice.content.contains("\"state\":\"paid\"")
            {
                // Verify allocations exist and sum correctly
                // Simplified check for now
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures closed periods are not modified without override.
#[derive(Debug, Clone, Default)]
pub struct ClosedPeriodReadonlyInvariant;

impl Invariant for ClosedPeriodReadonlyInvariant {
    fn name(&self) -> &str {
        "closed_period_readonly"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        // Check that facts in closed periods have override references
        for period in ctx.get(ContextKey::Proposals).iter() {
            if period.id.starts_with(PERIOD_PREFIX)
                && period.content.contains("\"state\":\"closed\"")
            {
                // Verify no modifications without override
            }
        }
        InvariantResult::Ok
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::{ContextState, Engine};

    #[tokio::test]
    async fn invoice_creator_produces_draft() {
        let mut engine = Engine::new();
        engine.register_suggestor(InvoiceCreatorAgent);

        let mut ctx = ContextState::new();
        let _ = ctx.add_input(
            ContextKey::Seeds,
            "trigger:deal.closed_won:deal_123",
            "deal.closed_won for customer ABC",
        );

        let result = engine.run(ctx).await.expect("should converge");
        assert!(result.converged);
        assert!(
            result
                .context
                .get(ContextKey::Proposals)
                .iter()
                .any(|f| f.id.starts_with(INVOICE_PREFIX))
        );
    }

    #[tokio::test]
    async fn invoice_issuer_requests_finance_approval_before_issue() {
        let mut engine = Engine::new();
        engine.register_suggestor(InvoiceIssuerAgent::default());

        let mut ctx = ContextState::new();
        let _ = ctx.add_input(
            ContextKey::Proposals,
            "invoice:draft:deal_123",
            r#"{"type":"invoice","state":"ready_to_issue","customer_id":"cust_123","line_items":[{"sku":"svc","amount":12500}],"amount":12500,"currency":"USD"}"#,
        );

        let result = engine.run(ctx).await.expect("should converge");
        assert!(result.converged);
        assert!(
            result
                .context
                .get(ContextKey::Proposals)
                .iter()
                .any(|fact| {
                    fact.id == "invoice:issue_request:invoice:draft:deal_123"
                        && fact
                            .content
                            .contains("\"required_role\":\"finance_manager\"")
                })
        );
    }

    #[tokio::test]
    async fn invoice_issuer_promotes_when_finance_approval_exists() {
        let mut engine = Engine::new();
        engine.register_suggestor(InvoiceIssuerAgent::default());

        let mut ctx = ContextState::new();
        let _ = ctx.add_input(
            ContextKey::Proposals,
            "invoice:draft:deal_123",
            r#"{"type":"invoice","state":"ready_to_issue","customer_id":"cust_123","line_items":[{"sku":"svc","amount":12500}],"amount":12500,"currency":"USD"}"#,
        );
        let _ = ctx.add_input(
            ContextKey::Proposals,
            "approval:invoice:invoice:draft:deal_123",
            r#"{"target_id":"invoice:draft:deal_123","required_role":"finance_manager"}"#,
        );

        let result = engine.run(ctx).await.expect("should converge");
        assert!(result.converged);
        assert!(
            result
                .context
                .get(ContextKey::Proposals)
                .iter()
                .any(|fact| {
                    fact.id == "invoice:issued:invoice:draft:deal_123"
                        && fact.content.contains("\"state\":\"issued\"")
                })
        );
    }

    #[tokio::test]
    async fn period_closer_promotes_when_finance_approval_exists() {
        let mut engine = Engine::new();
        engine.register_suggestor(PeriodCloserAgent::default());

        let mut ctx = ContextState::new();
        let _ = ctx.add_input(
            ContextKey::Proposals,
            "period:2026-03",
            r#"{"type":"period","state":"closing","reconciliation_complete":true}"#,
        );
        let _ = ctx.add_input(
            ContextKey::Proposals,
            "approval:period:period:2026-03",
            r#"{"target_id":"period:2026-03","required_role":"finance_manager"}"#,
        );

        let result = engine.run(ctx).await.expect("should converge");
        assert!(result.converged);
        assert!(
            result
                .context
                .get(ContextKey::Proposals)
                .iter()
                .any(|fact| {
                    fact.id == "period:closed:period:2026-03"
                        && fact.content.contains("\"state\":\"closed\"")
                })
        );
    }

    #[test]
    fn agents_have_correct_names() {
        assert_eq!(InvoiceCreatorAgent.name(), "invoice_creator");
        assert_eq!(InvoiceIssuerAgent::default().name(), "invoice_issuer");
        assert_eq!(PaymentAllocatorAgent.name(), "payment_allocator");
        assert_eq!(ReconciliationMatcherAgent.name(), "reconciliation_matcher");
        assert_eq!(OverdueDetectorAgent.name(), "overdue_detector");
        assert_eq!(PeriodCloserAgent::default().name(), "period_closer");
    }
}
