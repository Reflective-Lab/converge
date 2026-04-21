// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Expense Approval Workflow — multi-level approval with HITL gates.
//!
//! Demonstrates: long-running workflows, humans in the loop, and Cedar-backed
//! gate decisions projected from flow state.

use converge_kernel::{
    AgentEffect, AuthorityLevel, Context, ContextKey, ContextState, Engine, EngineHitlPolicy,
    FlowAction, FlowGateAuthorizer, FlowGateContext, FlowGateInput, FlowGateOutcome,
    FlowGatePrincipal, FlowGateResource, FlowPhase, GateDecision, ProposedFact, RunResult,
    Suggestor, TimeoutAction, TimeoutPolicy,
};
use converge_policy::PolicyEngine;
use std::path::PathBuf;
use std::sync::Arc;

struct ExpenseParsingAgent;

fn parse_expense(value: &str) -> serde_json::Value {
    serde_json::from_str(value).unwrap_or_default()
}

fn receipt_attached(expense: &serde_json::Value) -> bool {
    expense
        .get("receipt_attached")
        .and_then(|value| value.as_bool())
        .unwrap_or(true)
}

fn expense_amount(expense: &serde_json::Value) -> i64 {
    expense
        .get("amount")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0) as i64
}

fn has_human_approval(ctx: &dyn Context) -> bool {
    ctx.get(ContextKey::Proposals)
        .iter()
        .any(|fact| fact.id.ends_with("-approval"))
}

fn expense_policy_input(
    expense: &serde_json::Value,
    action: FlowAction,
    human_approval_present: bool,
) -> FlowGateInput {
    let mut gates_passed = Vec::new();
    if receipt_attached(expense) {
        gates_passed.push("receipt".to_string());
    }
    if human_approval_present {
        gates_passed.push("manager_approval".to_string());
    }

    FlowGateInput {
        principal: FlowGatePrincipal {
            id: "agent:finance-supervisor".into(),
            authority: AuthorityLevel::Supervisory,
            domains: vec!["finance".into()],
            policy_version: Some("expense_v1".into()),
        },
        resource: FlowGateResource {
            id: "expense:demo-001".into(),
            kind: "expense".into(),
            phase: FlowPhase::Commitment,
            gates_passed: gates_passed.into_iter().map(Into::into).collect(),
        },
        action,
        context: FlowGateContext {
            commitment_type: Some("expense".into()),
            amount: Some(expense_amount(expense)),
            human_approval_present: Some(human_approval_present),
            required_gates_met: Some(receipt_attached(expense)),
        },
    }
}

fn load_expense_policy_engine() -> Arc<dyn FlowGateAuthorizer> {
    let policy_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/policy/policies/expense_approval.cedar");
    let policy = std::fs::read_to_string(policy_path)
        .expect("expense approval Cedar policy should exist in converge-policy");
    Arc::new(PolicyEngine::from_policy_str(&policy).expect("expense approval policy should parse"))
}

#[async_trait::async_trait]
impl Suggestor for ExpenseParsingAgent {
    fn name(&self) -> &str {
        "ExpenseParsingAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Seeds) && !ctx.has(ContextKey::Strategies)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let seed = seeds.first();

        let parsed = if let Some(s) = seed {
            let json: serde_json::Value = serde_json::from_str(&s.content).unwrap_or_default();
            ProposedFact::new(
                ContextKey::Strategies,
                "parsed-expense",
                serde_json::to_string(&json).unwrap_or_default(),
                self.name(),
            )
            .with_confidence(1.0)
        } else {
            ProposedFact::new(ContextKey::Strategies, "parsed-expense", "{}", self.name())
                .with_confidence(1.0)
        };

        AgentEffect::with_proposals(vec![parsed])
    }
}

struct PolicyValidationAgent {
    policy: Arc<dyn FlowGateAuthorizer>,
}

#[async_trait::async_trait]
impl Suggestor for PolicyValidationAgent {
    fn name(&self) -> &str {
        "PolicyValidationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Strategies)
            && !ctx
                .get(ContextKey::Evaluations)
                .iter()
                .any(|fact| fact.id == "expense-validate-policy")
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let strategy = strategies.first();

        let result = strategy
            .map(|fact| parse_expense(&fact.content))
            .map(|expense| {
                let decision = self
                    .policy
                    .decide(&expense_policy_input(&expense, FlowAction::Validate, false))
                    .expect("policy evaluation should succeed for expense validation");

                serde_json::json!({
                    "gate": "validate",
                    "outcome": decision.outcome,
                    "reason": decision.reason,
                    "amount": expense_amount(&expense),
                    "receipt_attached": receipt_attached(&expense)
                })
            })
            .unwrap_or_else(|| {
                serde_json::json!({
                    "gate": "validate",
                    "outcome": FlowGateOutcome::Reject,
                    "reason": "missing parsed expense"
                })
            });

        AgentEffect::with_proposal(
            ProposedFact::new(
                ContextKey::Evaluations,
                "expense-validate-policy",
                result.to_string(),
                self.name(),
            )
            .with_confidence(1.0),
        )
    }
}

struct ApprovalRoutingAgent {
    policy: Arc<dyn FlowGateAuthorizer>,
}

const ROUTING_DEPS: [ContextKey; 2] = [ContextKey::Strategies, ContextKey::Evaluations];

#[async_trait::async_trait]
impl Suggestor for ApprovalRoutingAgent {
    fn name(&self) -> &str {
        "ApprovalRoutingAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &ROUTING_DEPS
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Strategies)
            && ctx
                .get(ContextKey::Evaluations)
                .iter()
                .any(|fact| fact.id == "expense-validate-policy")
            && !ctx
                .get(ContextKey::Constraints)
                .iter()
                .any(|fact| fact.id == "expense-approval-routing")
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let evaluations = ctx.get(ContextKey::Evaluations);
        let strategies = ctx.get(ContextKey::Strategies);

        if let (Some(e), Some(s)) = (evaluations.first(), strategies.first()) {
            let eval: serde_json::Value = serde_json::from_str(&e.content).unwrap_or_default();
            let expense = parse_expense(&s.content);
            let validate_outcome = eval
                .get("outcome")
                .and_then(|value| value.as_str())
                .unwrap_or("reject");

            let commit_decision = self
                .policy
                .decide(&expense_policy_input(&expense, FlowAction::Commit, false))
                .expect("policy evaluation should succeed for commit routing");

            let (required_approvers, current_approver) = match commit_decision.outcome {
                FlowGateOutcome::Escalate => (vec!["manager".to_string()], Some("manager")),
                FlowGateOutcome::Reject if validate_outcome != "promote" => {
                    (vec!["finance".to_string()], Some("finance"))
                }
                FlowGateOutcome::Reject => (vec!["finance".to_string()], Some("finance")),
                FlowGateOutcome::Promote => (Vec::new(), None),
            };

            let routing = serde_json::json!({
                "required_approvers": required_approvers,
                "current_approver": current_approver,
                "pending": if current_approver.is_some() { 1 } else { 0 },
                "validate_outcome": validate_outcome,
                "commit_outcome": commit_decision.outcome,
                "commit_reason": commit_decision.reason
            });

            return AgentEffect::with_proposal(
                ProposedFact::new(
                    ContextKey::Constraints,
                    "expense-approval-routing",
                    routing.to_string(),
                    self.name(),
                )
                .with_confidence(1.0),
            );
        }

        AgentEffect::default()
    }
}

struct CommitDecisionAgent {
    policy: Arc<dyn FlowGateAuthorizer>,
}

const COMMIT_DEPS: [ContextKey; 3] = [
    ContextKey::Strategies,
    ContextKey::Constraints,
    ContextKey::Proposals,
];

#[async_trait::async_trait]
impl Suggestor for CommitDecisionAgent {
    fn name(&self) -> &str {
        "CommitDecisionAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &COMMIT_DEPS
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Strategies)
            && ctx
                .get(ContextKey::Constraints)
                .iter()
                .any(|fact| fact.id == "expense-approval-routing")
            && !ctx
                .get(ContextKey::Evaluations)
                .iter()
                .any(|fact| fact.id == "expense-commit-policy")
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let Some(strategy) = ctx.get(ContextKey::Strategies).first() else {
            return AgentEffect::default();
        };

        let expense = parse_expense(&strategy.content);
        let human_approval_present = has_human_approval(ctx);
        let constraint = ctx
            .get(ContextKey::Constraints)
            .iter()
            .find(|fact| fact.id == "expense-approval-routing");

        if !human_approval_present {
            let pending = constraint
                .and_then(|fact| serde_json::from_str::<serde_json::Value>(&fact.content).ok())
                .and_then(|json| json.get("pending").and_then(|value| value.as_u64()))
                .unwrap_or(0);
            if pending > 0 {
                return AgentEffect::default();
            }
        }

        let decision = self
            .policy
            .decide(&expense_policy_input(
                &expense,
                FlowAction::Commit,
                human_approval_present,
            ))
            .expect("policy evaluation should succeed for final commit");

        let result = serde_json::json!({
            "gate": "commit",
            "outcome": decision.outcome,
            "reason": decision.reason,
            "human_approval_present": human_approval_present
        });

        AgentEffect::with_proposal(
            ProposedFact::new(
                ContextKey::Evaluations,
                "expense-commit-policy",
                result.to_string(),
                self.name(),
            )
            .with_confidence(1.0),
        )
    }
}

struct ApprovalSimulationAgent;

#[async_trait::async_trait]
impl Suggestor for ApprovalSimulationAgent {
    fn name(&self) -> &str {
        "ApprovalSimulationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Constraints]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.get(ContextKey::Constraints)
            .iter()
            .any(|fact| fact.id == "expense-approval-routing")
            && !has_human_approval(ctx)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        if let Some(c) = ctx
            .get(ContextKey::Constraints)
            .iter()
            .find(|fact| fact.id == "expense-approval-routing")
        {
            let routing: serde_json::Value = serde_json::from_str(&c.content).unwrap_or_default();
            let pending = routing
                .get("pending")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            if pending == 0 {
                return AgentEffect::default();
            }

            let current = routing
                .get("current_approver")
                .and_then(|v| v.as_str())
                .unwrap_or("manager");

            let proposal = ProposedFact {
                key: ContextKey::Proposals,
                id: format!("{current}-approval").into(),
                content: format!("Approved by {current}"),
                confidence: 0.95,
                provenance: format!("{current} approval agent"),
            };

            return AgentEffect::with_proposal(proposal);
        }

        AgentEffect::default()
    }
}

#[tokio::main]
async fn main() {
    println!("=== Expense Approval Workflow Example ===\n");

    let mut engine = Engine::new();
    let policy = load_expense_policy_engine();

    engine.register_suggestor(ExpenseParsingAgent);
    engine.register_suggestor(PolicyValidationAgent {
        policy: Arc::clone(&policy),
    });
    engine.register_suggestor(ApprovalRoutingAgent {
        policy: Arc::clone(&policy),
    });
    engine.register_suggestor(ApprovalSimulationAgent);
    engine.register_suggestor(CommitDecisionAgent { policy });

    let hitl_policy = EngineHitlPolicy {
        confidence_threshold: Some(0.8),
        gated_keys: vec![ContextKey::Proposals],
        timeout: TimeoutPolicy {
            timeout_secs: 300,
            action: TimeoutAction::Reject,
        },
    };
    engine.set_hitl_policy(hitl_policy);

    let expense = serde_json::json!({
        "employee": "john.doe@example.com",
        "amount": 4200.00,
        "category": "entertainment",
        "description": "Client dinner",
        "date": "2026-04-15",
        "receipt_attached": true
    });

    let mut ctx = ContextState::new();
    let _ = ctx.add_input(ContextKey::Seeds, "expense-1", expense.to_string());

    println!(
        "Expense submitted: ${} {} - {}\n",
        expense["amount"], expense["category"], expense["description"]
    );
    println!("Running approval workflow...\n");

    match engine.run_with_hitl(ctx).await {
        RunResult::HitlPause(pause) => {
            println!("⏸️  HITL Gate: Cedar required human approval");
            println!("    Proposal: {}", pause.request.summary);
            println!(
                "    Approver: {}",
                pause.request.rationale.as_deref().unwrap_or("manager")
            );
            println!();

            let decision =
                GateDecision::approve(pause.request.gate_id.clone(), "manager@company.com");

            println!("▶️  Manager approved. Resuming workflow...\n");

            match engine.resume(*pause, decision).await {
                RunResult::Complete(Ok(result)) => {
                    println!("✅ Expense flow completed.\n");
                    for fact in result.context.get(ContextKey::Evaluations) {
                        println!("  [{}] {}", fact.id, fact.content);
                    }
                }
                RunResult::HitlPause(_) => println!("❌ Unexpected extra approval stage"),
                _ => println!("❌ Approval workflow failed"),
            }
        }
        RunResult::Complete(Ok(result)) => {
            println!("✅ Expense flow completed without HITL.\n");
            for fact in result.context.get(ContextKey::Evaluations) {
                println!("  [{}] {}", fact.id, fact.content);
            }
        }
        RunResult::Complete(Err(e)) => {
            println!("❌ Workflow failed: {e}");
        }
    }

    println!("\n=== Done ===");
}
