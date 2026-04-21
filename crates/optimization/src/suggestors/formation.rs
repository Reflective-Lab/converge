// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Formation assembly via bipartite matching.
//!
//! Reads a [`FormationRequest`] from context, matches required roles against a
//! catalog of [`ProfileSnapshot`]s using Hopcroft-Karp, and proposes a
//! [`FormationPlan`] to [`ContextKey::Strategies`].

use async_trait::async_trait;
use converge_core::profile::{ProfileSnapshot, SuggestorCapability, SuggestorRole};
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use serde::{Deserialize, Serialize};

use crate::graph::matching::bipartite_matching;

// ── Request ───────────────────────────────────────────────────────────────────

/// A formation request seeded into context.
///
/// Place this as a JSON-serialised fact under [`ContextKey::Seeds`] with an id
/// prefixed `"formation-request:"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationRequest {
    /// Stable identifier for this request (used to detect idempotency).
    pub id: String,
    /// Roles that must be covered.  Duplicates request multiple instances.
    pub required_roles: Vec<SuggestorRole>,
    /// Additional capability constraints applied after role matching.
    pub required_capabilities: Vec<SuggestorCapability>,
}

// ── Plan (output) ─────────────────────────────────────────────────────────────

/// The proposed formation plan produced by the suggestor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationPlan {
    /// The request this plan answers.
    pub request_id: String,
    /// Matched (role, suggestor name) pairs.
    pub assignments: Vec<RoleAssignment>,
    /// Roles that could not be filled from the catalog.
    pub unmatched_roles: Vec<SuggestorRole>,
    /// `assignments.len() / required_roles.len()` — 1.0 is full coverage.
    pub coverage_ratio: f64,
}

/// A single role-to-suggestor assignment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleAssignment {
    pub role: SuggestorRole,
    pub suggestor: String,
}

// ── Suggestor ─────────────────────────────────────────────────────────────────

const REQUEST_PREFIX: &str = "formation-request:";
const PLAN_PREFIX: &str = "formation-plan:";

/// Assembles a formation by matching required roles to available suggestors.
///
/// # Construction
///
/// ```rust,ignore
/// let catalog: Vec<ProfileSnapshot> = engine
///     .registered_suggestors()
///     .map(|(name, profile)| ProfileSnapshot::from_profile(name, profile))
///     .collect();
///
/// engine.register_suggestor(FormationAssemblySuggestor::new(catalog));
/// ```
pub struct FormationAssemblySuggestor {
    catalog: Vec<ProfileSnapshot>,
}

impl FormationAssemblySuggestor {
    pub fn new(catalog: Vec<ProfileSnapshot>) -> Self {
        Self { catalog }
    }
}

#[async_trait]
impl Suggestor for FormationAssemblySuggestor {
    fn name(&self) -> &str {
        "FormationAssemblySuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|f| f.id.starts_with(REQUEST_PREFIX) && !plan_exists(ctx, request_id(&f.id)))
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let requests: Vec<FormationRequest> = ctx
            .get(ContextKey::Seeds)
            .iter()
            .filter(|f| f.id.starts_with(REQUEST_PREFIX))
            .filter(|f| !plan_exists(ctx, request_id(&f.id)))
            .filter_map(|f| serde_json::from_str(&f.content).ok())
            .collect();

        if requests.is_empty() {
            return AgentEffect::empty();
        }

        let proposals: Vec<ProposedFact> = requests
            .into_iter()
            .map(|req| assemble(&req, &self.catalog))
            .map(|plan| {
                ProposedFact::new(
                    ContextKey::Strategies,
                    format!("{}{}", PLAN_PREFIX, plan.request_id),
                    serde_json::to_string(&plan).unwrap_or_default(),
                    self.name(),
                )
                .with_confidence(plan.coverage_ratio)
            })
            .collect();

        AgentEffect::with_proposals(proposals)
    }
}

// ── Matching logic ────────────────────────────────────────────────────────────

fn assemble(req: &FormationRequest, catalog: &[ProfileSnapshot]) -> FormationPlan {
    // Filter catalog to suggestors that satisfy all required_capabilities (if any).
    let eligible: Vec<&ProfileSnapshot> = if req.required_capabilities.is_empty() {
        catalog.iter().collect()
    } else {
        catalog
            .iter()
            .filter(|s| {
                req.required_capabilities
                    .iter()
                    .all(|cap| s.capabilities.contains(cap))
            })
            .collect()
    };

    // Left = required role slots (index = position in req.required_roles).
    // Right = eligible catalog entries (index = position in `eligible`).
    // Edge: eligible[j].role == req.required_roles[i].
    let edges: Vec<(usize, usize)> = req
        .required_roles
        .iter()
        .enumerate()
        .flat_map(|(i, role)| {
            eligible
                .iter()
                .enumerate()
                .filter(move |(_, s)| s.role == *role)
                .map(move |(j, _)| (i, j))
        })
        .collect();

    let matching =
        bipartite_matching(req.required_roles.len(), eligible.len(), &edges).unwrap_or_default();

    let mut assigned = vec![false; req.required_roles.len()];
    let mut assignments = Vec::with_capacity(matching.size);

    for (role_idx, cand_idx) in &matching.pairs {
        assignments.push(RoleAssignment {
            role: req.required_roles[*role_idx],
            suggestor: eligible[*cand_idx].name.clone(),
        });
        assigned[*role_idx] = true;
    }

    let unmatched_roles = req
        .required_roles
        .iter()
        .enumerate()
        .filter(|(i, _)| !assigned[*i])
        .map(|(_, r)| *r)
        .collect::<Vec<_>>();

    let coverage_ratio = if req.required_roles.is_empty() {
        1.0
    } else {
        matching.size as f64 / req.required_roles.len() as f64
    };

    FormationPlan {
        request_id: req.id.clone(),
        assignments,
        unmatched_roles,
        coverage_ratio,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn request_id(fact_id: &str) -> &str {
    fact_id.trim_start_matches(REQUEST_PREFIX)
}

fn plan_exists(ctx: &dyn Context, request_id: &str) -> bool {
    let plan_id = format!("{}{}", PLAN_PREFIX, request_id);
    ctx.get(ContextKey::Strategies)
        .iter()
        .any(|f| f.id == plan_id)
}

// ── Default for Matching (graceful degradation) ───────────────────────────────

impl Default for crate::graph::matching::Matching {
    fn default() -> Self {
        Self {
            pairs: vec![],
            size: 0,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::profile::SuggestorCapability;
    use converge_core::{CostClass, LatencyClass};
    use converge_pack::ContextKey;

    fn snapshot(name: &str, role: SuggestorRole, caps: &[SuggestorCapability]) -> ProfileSnapshot {
        ProfileSnapshot {
            name: name.to_string(),
            role,
            output_keys: vec![ContextKey::Strategies],
            cost_hint: CostClass::Medium,
            latency_hint: LatencyClass::Interactive,
            capabilities: caps.to_vec(),
            confidence_min: 0.5,
            confidence_max: 0.95,
        }
    }

    fn request(
        id: &str,
        roles: &[SuggestorRole],
        caps: &[SuggestorCapability],
    ) -> FormationRequest {
        FormationRequest {
            id: id.to_string(),
            required_roles: roles.to_vec(),
            required_capabilities: caps.to_vec(),
        }
    }

    #[test]
    fn full_coverage_when_catalog_satisfies_all_roles() {
        let catalog = vec![
            snapshot("analyser", SuggestorRole::Analysis, &[]),
            snapshot("planner", SuggestorRole::Planning, &[]),
            snapshot("enforcer", SuggestorRole::Constraint, &[]),
        ];
        let req = request(
            "r1",
            &[
                SuggestorRole::Analysis,
                SuggestorRole::Planning,
                SuggestorRole::Constraint,
            ],
            &[],
        );

        let plan = assemble(&req, &catalog);

        assert_eq!(plan.assignments.len(), 3);
        assert!(plan.unmatched_roles.is_empty());
        assert!((plan.coverage_ratio - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn partial_coverage_when_catalog_missing_a_role() {
        let catalog = vec![
            snapshot("analyser", SuggestorRole::Analysis, &[]),
            snapshot("planner", SuggestorRole::Planning, &[]),
        ];
        let req = request(
            "r2",
            &[
                SuggestorRole::Analysis,
                SuggestorRole::Planning,
                SuggestorRole::Constraint,
            ],
            &[],
        );

        let plan = assemble(&req, &catalog);

        assert_eq!(plan.assignments.len(), 2);
        assert_eq!(plan.unmatched_roles, vec![SuggestorRole::Constraint]);
        assert!((plan.coverage_ratio - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn capability_filter_excludes_ineligible_suggestors() {
        let catalog = vec![
            snapshot(
                "llm-analyser",
                SuggestorRole::Analysis,
                &[SuggestorCapability::LlmReasoning],
            ),
            snapshot("plain-analyser", SuggestorRole::Analysis, &[]),
        ];
        // Only LlmReasoning-capable suggestors should be considered.
        let req = request(
            "r3",
            &[SuggestorRole::Analysis],
            &[SuggestorCapability::LlmReasoning],
        );

        let plan = assemble(&req, &catalog);

        assert_eq!(plan.assignments.len(), 1);
        assert_eq!(plan.assignments[0].suggestor, "llm-analyser");
    }

    #[test]
    fn no_double_booking_with_two_same_role_slots() {
        let catalog = vec![
            snapshot("a1", SuggestorRole::Analysis, &[]),
            snapshot("a2", SuggestorRole::Analysis, &[]),
        ];
        let req = request(
            "r4",
            &[SuggestorRole::Analysis, SuggestorRole::Analysis],
            &[],
        );

        let plan = assemble(&req, &catalog);

        assert_eq!(plan.assignments.len(), 2);
        // Each suggestor used at most once.
        let names: Vec<_> = plan.assignments.iter().map(|a| &a.suggestor).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(unique.len(), 2);
    }

    #[test]
    fn empty_catalog_yields_zero_coverage() {
        let req = request(
            "r5",
            &[SuggestorRole::Analysis, SuggestorRole::Planning],
            &[],
        );
        let plan = assemble(&req, &[]);
        assert_eq!(plan.assignments.len(), 0);
        assert_eq!(plan.coverage_ratio, 0.0);
    }

    #[test]
    fn empty_request_yields_full_coverage() {
        let catalog = vec![snapshot("a", SuggestorRole::Analysis, &[])];
        let req = request("r6", &[], &[]);
        let plan = assemble(&req, &catalog);
        assert_eq!(plan.assignments.len(), 0);
        assert!((plan.coverage_ratio - 1.0).abs() < f64::EPSILON);
    }
}
