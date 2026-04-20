// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Delivery Pack agents for promise fulfillment.
//!
//! Implements the agent contracts defined in specs/delivery.feature.
//!
//! # Lifecycle: Promise → Execute → Blockers → Complete → Learn
//!
//! Note: This implementation uses the standard ContextKey enum. Facts are
//! distinguished by their ID prefixes (promise:, task:, blocker:, etc.).

use converge_core::{
    AgentEffect, ContextKey, Suggestor,
    invariant::{Invariant, InvariantClass, InvariantResult, Violation},
};

// ============================================================================
// Fact ID Prefixes
// ============================================================================

pub const PROMISE_PREFIX: &str = "promise:";
pub const SCOPE_PREFIX: &str = "scope:";
pub const TASK_PREFIX: &str = "task:";
pub const BLOCKER_PREFIX: &str = "blocker:";
pub const RISK_PREFIX: &str = "risk:";
pub const ACCEPTANCE_PREFIX: &str = "acceptance:";
pub const POSTMORTEM_PREFIX: &str = "postmortem:";

// ============================================================================
// Agents
// ============================================================================

/// Creates delivery promises from closed deals.
#[derive(Debug, Clone, Default)]
pub struct PromiseCreatorAgent;

#[async_trait::async_trait]
impl Suggestor for PromiseCreatorAgent {
    fn name(&self) -> &str {
        "promise_creator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("deal.closed_won"))
            && !ctx
                .get(ContextKey::Proposals)
                .iter()
                .any(|p| p.id.starts_with(PROMISE_PREFIX))
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("deal.closed_won") {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{}{}", PROMISE_PREFIX, trigger.id),
                    serde_json::json!({
                        "type": "promise",
                        "deal_id": trigger.id,
                        "state": "draft",
                        "customer_id": "extracted",
                        "created_at": "2026-01-12"
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Extracts scope from contract terms.
#[derive(Debug, Clone, Default)]
pub struct ScopeExtractorAgent;

#[async_trait::async_trait]
impl Suggestor for ScopeExtractorAgent {
    fn name(&self) -> &str {
        "scope_extractor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(PROMISE_PREFIX) && p.content.contains("\"state\":\"draft\""))
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for promise in proposals.iter() {
            if promise.id.starts_with(PROMISE_PREFIX)
                && promise.content.contains("\"state\":\"draft\"")
            {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{}{}", SCOPE_PREFIX, promise.id),
                    serde_json::json!({
                        "type": "scope",
                        "promise_id": promise.id,
                        "deliverables": [],
                        "success_criteria": [],
                        "timeline": "30 days"
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Breaks down scope into executable tasks.
#[derive(Debug, Clone, Default)]
pub struct WorkBreakdownAgent;

#[async_trait::async_trait]
impl Suggestor for WorkBreakdownAgent {
    fn name(&self) -> &str {
        "work_breakdown"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        let has_scope = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(SCOPE_PREFIX));
        let has_tasks = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(TASK_PREFIX));
        has_scope && !has_tasks
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for item in proposals.iter() {
            if item.id.starts_with(SCOPE_PREFIX) {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{}1:{}", TASK_PREFIX, item.id),
                    serde_json::json!({
                        "type": "task",
                        "work_item_id": item.id,
                        "title": "Implementation task",
                        "state": "pending",
                        "estimated_hours": 40
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Detects blockers in task execution.
#[derive(Debug, Clone, Default)]
pub struct BlockerDetectorAgent;

#[async_trait::async_trait]
impl Suggestor for BlockerDetectorAgent {
    fn name(&self) -> &str {
        "blocker_detector"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|t| t.id.starts_with(TASK_PREFIX) && t.content.contains("\"blocked\":true"))
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for task in proposals.iter() {
            if task.id.starts_with(TASK_PREFIX) && task.content.contains("\"blocked\":true") {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{}{}", BLOCKER_PREFIX, task.id),
                    serde_json::json!({
                        "type": "blocker",
                        "task_id": task.id,
                        "state": "raised",
                        "severity": "medium",
                        "description": "Task is blocked"
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Routes blockers to appropriate owners.
#[derive(Debug, Clone, Default)]
pub struct BlockerRouterAgent;

#[async_trait::async_trait]
impl Suggestor for BlockerRouterAgent {
    fn name(&self) -> &str {
        "blocker_router"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|b| b.id.starts_with(BLOCKER_PREFIX) && b.content.contains("\"state\":\"raised\""))
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for blocker in proposals.iter() {
            if blocker.id.starts_with(BLOCKER_PREFIX)
                && blocker.content.contains("\"state\":\"raised\"")
            {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{}routed:{}", BLOCKER_PREFIX, blocker.id),
                    serde_json::json!({
                        "type": "routed_blocker",
                        "blocker_id": blocker.id,
                        "state": "assigned",
                        "owner": "tech_lead",
                        "sla_hours": 24
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Assesses risk for promises.
#[derive(Debug, Clone, Default)]
pub struct RiskAssessorAgent;

#[async_trait::async_trait]
impl Suggestor for RiskAssessorAgent {
    fn name(&self) -> &str {
        "risk_assessor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        let has_promises = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(PROMISE_PREFIX));
        let has_risks = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|r| r.id.starts_with(RISK_PREFIX));
        has_promises && !has_risks
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let blocker_count = proposals
            .iter()
            .filter(|p| p.id.starts_with(BLOCKER_PREFIX))
            .count();

        let mut facts = Vec::new();

        for promise in proposals.iter() {
            if promise.id.starts_with(PROMISE_PREFIX) {
                let risk_level = if blocker_count > 2 {
                    "high"
                } else if blocker_count > 0 {
                    "medium"
                } else {
                    "low"
                };

                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Evaluations,
                    format!("{}{}", RISK_PREFIX, promise.id),
                    serde_json::json!({
                        "type": "risk_assessment",
                        "promise_id": promise.id,
                        "risk_level": risk_level,
                        "blocker_count": blocker_count,
                        "mitigation_required": risk_level != "low"
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Aggregates status across promise components.
#[derive(Debug, Clone, Default)]
pub struct StatusAggregatorAgent;

#[async_trait::async_trait]
impl Suggestor for StatusAggregatorAgent {
    fn name(&self) -> &str {
        "status_aggregator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        let has_promises = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(PROMISE_PREFIX));
        let has_tasks = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(TASK_PREFIX));
        has_promises && has_tasks
    }

    async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
        // Aggregates status - simplified implementation
        AgentEffect::empty()
    }
}

/// Requests acceptance from customers.
#[derive(Debug, Clone, Default)]
pub struct AcceptanceRequestorAgent;

#[async_trait::async_trait]
impl Suggestor for AcceptanceRequestorAgent {
    fn name(&self) -> &str {
        "acceptance_requestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(PROMISE_PREFIX) && p.content.contains("\"state\":\"review\""))
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for promise in proposals.iter() {
            if promise.id.starts_with(PROMISE_PREFIX)
                && promise.content.contains("\"state\":\"review\"")
            {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{}{}", ACCEPTANCE_PREFIX, promise.id),
                    serde_json::json!({
                        "type": "acceptance_request",
                        "promise_id": promise.id,
                        "state": "pending",
                        "requested_at": "2026-01-12"
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Schedules postmortems for completed promises.
#[derive(Debug, Clone, Default)]
pub struct PostmortemSchedulerAgent;

#[async_trait::async_trait]
impl Suggestor for PostmortemSchedulerAgent {
    fn name(&self) -> &str {
        "postmortem_scheduler"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(PROMISE_PREFIX) && p.content.contains("\"state\":\"completed\"")
        })
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for promise in proposals.iter() {
            if promise.id.starts_with(PROMISE_PREFIX)
                && promise.content.contains("\"state\":\"completed\"")
            {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{}{}", POSTMORTEM_PREFIX, promise.id),
                    serde_json::json!({
                        "type": "postmortem",
                        "promise_id": promise.id,
                        "scheduled_for": "2026-01-19",
                        "participants": ["delivery_team", "customer_success"]
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

// ============================================================================
// Invariants
// ============================================================================

/// Ensures promises have linked deals.
#[derive(Debug, Clone, Default)]
pub struct PromiseHasDealInvariant;

impl Invariant for PromiseHasDealInvariant {
    fn name(&self) -> &str {
        "promise_has_deal"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        for promise in ctx.get(ContextKey::Proposals).iter() {
            if promise.id.starts_with(PROMISE_PREFIX) && !promise.content.contains("\"deal_id\":") {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Promise {} missing deal_id", promise.id),
                    vec![promise.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures raised blockers have a resolution path (assigned owner).
#[derive(Debug, Clone, Default)]
pub struct BlockerHasResolutionPathInvariant;

impl Invariant for BlockerHasResolutionPathInvariant {
    fn name(&self) -> &str {
        "blocker_has_resolution_path"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        let proposals = ctx.get(ContextKey::Proposals);

        for blocker in proposals.iter() {
            // Check blockers that are raised but not yet routed
            if blocker.id.starts_with(BLOCKER_PREFIX)
                && blocker.content.contains("\"state\":\"raised\"")
            {
                // Look for a corresponding routed blocker
                let routed_id = format!("{}routed:{}", BLOCKER_PREFIX, blocker.id);
                let has_route = proposals.iter().any(|p| p.id == routed_id);

                if !has_route && !blocker.content.contains("\"owner\":") {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!(
                            "Blocker {} has no resolution path or owner assigned",
                            blocker.id
                        ),
                        vec![blocker.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures scope changes require explicit approval.
#[derive(Debug, Clone, Default)]
pub struct ScopeChangeRequiresApprovalInvariant;

impl Invariant for ScopeChangeRequiresApprovalInvariant {
    fn name(&self) -> &str {
        "scope_change_requires_approval"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        for scope in ctx.get(ContextKey::Proposals).iter() {
            if scope.id.starts_with(SCOPE_PREFIX) && scope.content.contains("\"change_type\":") {
                // Scope has been modified - check for approval
                if !scope.content.contains("\"approved\":true")
                    && !scope.content.contains("\"approval_status\":\"approved\"")
                {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Scope change {} requires approval", scope.id),
                        vec![scope.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures promises can only be completed with customer acceptance.
#[derive(Debug, Clone, Default)]
pub struct CompletedPromiseHasAcceptanceInvariant;

impl Invariant for CompletedPromiseHasAcceptanceInvariant {
    fn name(&self) -> &str {
        "completed_promise_has_acceptance"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        let proposals = ctx.get(ContextKey::Proposals);

        for promise in proposals.iter() {
            if promise.id.starts_with(PROMISE_PREFIX)
                && promise.content.contains("\"state\":\"completed\"")
            {
                // Look for acceptance record
                let acceptance_id = format!("{}{}", ACCEPTANCE_PREFIX, promise.id);
                let has_acceptance = proposals
                    .iter()
                    .any(|p| p.id == acceptance_id && p.content.contains("\"state\":\"accepted\""));

                if !has_acceptance {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!(
                            "Promise {} marked complete without customer acceptance",
                            promise.id
                        ),
                        vec![promise.id.clone()],
                    ));
                }
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

    fn promoted_context(entries: &[(ContextKey, &str, &str)]) -> ContextState {
        let mut ctx = ContextState::new();
        for (key, id, content) in entries {
            let _ = ctx.add_input(*key, *id, *content);
        }
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(Engine::new().run(ctx))
            .expect("should promote test inputs")
            .context
    }

    #[test]
    fn agents_have_correct_names() {
        assert_eq!(PromiseCreatorAgent.name(), "promise_creator");
        assert_eq!(ScopeExtractorAgent.name(), "scope_extractor");
        assert_eq!(WorkBreakdownAgent.name(), "work_breakdown");
        assert_eq!(BlockerDetectorAgent.name(), "blocker_detector");
        assert_eq!(BlockerRouterAgent.name(), "blocker_router");
        assert_eq!(RiskAssessorAgent.name(), "risk_assessor");
        assert_eq!(StatusAggregatorAgent.name(), "status_aggregator");
        assert_eq!(AcceptanceRequestorAgent.name(), "acceptance_requestor");
        assert_eq!(PostmortemSchedulerAgent.name(), "postmortem_scheduler");
    }

    #[test]
    fn invariants_have_correct_names() {
        assert_eq!(PromiseHasDealInvariant.name(), "promise_has_deal");
        assert_eq!(
            BlockerHasResolutionPathInvariant.name(),
            "blocker_has_resolution_path"
        );
        assert_eq!(
            ScopeChangeRequiresApprovalInvariant.name(),
            "scope_change_requires_approval"
        );
        assert_eq!(
            CompletedPromiseHasAcceptanceInvariant.name(),
            "completed_promise_has_acceptance"
        );
    }

    #[test]
    fn blocker_without_resolution_path_violates() {
        let ctx = promoted_context(&[(
            ContextKey::Proposals,
            "blocker:task:123",
            r#"{"type":"blocker","state":"raised","severity":"high"}"#,
        )]);

        let result = BlockerHasResolutionPathInvariant.check(&ctx);
        assert!(matches!(result, InvariantResult::Violated(_)));
    }

    #[test]
    fn blocker_with_owner_passes() {
        let ctx = promoted_context(&[(
            ContextKey::Proposals,
            "blocker:task:123",
            r#"{"type":"blocker","state":"raised","owner":"tech_lead"}"#,
        )]);

        let result = BlockerHasResolutionPathInvariant.check(&ctx);
        assert!(matches!(result, InvariantResult::Ok));
    }

    #[test]
    fn scope_change_without_approval_violates() {
        let ctx = promoted_context(&[(
            ContextKey::Proposals,
            "scope:promise:123",
            r#"{"type":"scope","change_type":"addition","approved":false}"#,
        )]);

        let result = ScopeChangeRequiresApprovalInvariant.check(&ctx);
        assert!(matches!(result, InvariantResult::Violated(_)));
    }

    #[test]
    fn scope_change_with_approval_passes() {
        let ctx = promoted_context(&[(
            ContextKey::Proposals,
            "scope:promise:123",
            r#"{"type":"scope","change_type":"addition","approved":true}"#,
        )]);

        let result = ScopeChangeRequiresApprovalInvariant.check(&ctx);
        assert!(matches!(result, InvariantResult::Ok));
    }

    #[test]
    fn completed_promise_without_acceptance_violates() {
        let ctx = promoted_context(&[(
            ContextKey::Proposals,
            "promise:deal:123",
            r#"{"type":"promise","deal_id":"deal:123","state":"completed"}"#,
        )]);

        let result = CompletedPromiseHasAcceptanceInvariant.check(&ctx);
        assert!(matches!(result, InvariantResult::Violated(_)));
    }

    #[test]
    fn completed_promise_with_acceptance_passes() {
        let ctx = promoted_context(&[
            (
                ContextKey::Proposals,
                "promise:deal:123",
                r#"{"type":"promise","deal_id":"deal:123","state":"completed"}"#,
            ),
            (
                ContextKey::Proposals,
                "acceptance:promise:deal:123",
                r#"{"type":"acceptance_request","state":"accepted"}"#,
            ),
        ]);

        let result = CompletedPromiseHasAcceptanceInvariant.check(&ctx);
        assert!(matches!(result, InvariantResult::Ok));
    }
}
