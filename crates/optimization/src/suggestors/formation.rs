// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Formation assembly via bipartite matching.
//!
//! Reads a [`FormationRequest`] from context, matches required roles against a
//! catalog of [`ProfileSnapshot`]s using Hopcroft-Karp, and proposes a
//! [`FormationPlan`] to [`ContextKey::Strategies`].

use async_trait::async_trait;
use converge_model::formation::{FormationPlan, FormationRequest, ProfileSnapshot, RoleAssignment};
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};

use crate::graph::matching::bipartite_matching;

// ── Suggestor ─────────────────────────────────────────────────────────────────

const REQUEST_PREFIX: &str = "formation-request:";
const PLAN_PREFIX: &str = "formation-plan:";
const MALFORMED_PREFIX: &str = "formation-request-error:";

/// Assembles a formation by matching required roles to available suggestors.
///
/// # Construction
///
/// ```rust,ignore
/// let mut catalog = Vec::new();
///
/// register_profiled(&mut engine, &mut catalog, analysis_suggestor);
/// register_profiled(&mut engine, &mut catalog, planning_suggestor);
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
        ctx.get(ContextKey::Seeds).iter().any(|f| {
            f.id.starts_with(REQUEST_PREFIX)
                && match serde_json::from_str::<FormationRequest>(&f.content) {
                    Ok(_) => !plan_exists(ctx, request_id(&f.id)),
                    Err(_) => !malformed_diagnostic_exists(ctx, &f.id),
                }
        })
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut proposals = Vec::new();

        for fact in ctx
            .get(ContextKey::Seeds)
            .iter()
            .filter(|f| f.id.starts_with(REQUEST_PREFIX))
        {
            match serde_json::from_str::<FormationRequest>(&fact.content) {
                Ok(req) => {
                    if plan_exists(ctx, request_id(&fact.id)) {
                        continue;
                    }

                    let plan = assemble(&req, &self.catalog);
                    proposals.push(
                        ProposedFact::new(
                            ContextKey::Strategies,
                            format!("{}{}", PLAN_PREFIX, plan.request_id),
                            serde_json::to_string(&plan).unwrap_or_default(),
                            self.name(),
                        )
                        .with_confidence(plan.coverage_ratio),
                    );
                }
                Err(error) => {
                    if malformed_diagnostic_exists(ctx, &fact.id) {
                        continue;
                    }

                    let diagnostic = serde_json::json!({
                        "request_fact_id": fact.id,
                        "message": "malformed formation request ignored",
                        "error": error.to_string(),
                    });
                    proposals.push(
                        ProposedFact::new(
                            ContextKey::Diagnostic,
                            malformed_diagnostic_id(&fact.id),
                            diagnostic.to_string(),
                            self.name(),
                        )
                        .with_confidence(1.0),
                    );
                }
            }
        }

        if proposals.is_empty() {
            AgentEffect::empty()
        } else {
            AgentEffect::with_proposals(proposals)
        }
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

fn malformed_diagnostic_id(fact_id: &str) -> String {
    format!("{MALFORMED_PREFIX}{fact_id}")
}

fn malformed_diagnostic_exists(ctx: &dyn Context, fact_id: &str) -> bool {
    let diagnostic_id = malformed_diagnostic_id(fact_id);
    ctx.get(ContextKey::Diagnostic)
        .iter()
        .any(|fact| fact.id == diagnostic_id)
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
    use converge_core::{ContextState, Engine};
    use converge_model::formation::{SuggestorCapability, SuggestorRole};
    use converge_pack::ContextKey;
    use converge_provider_api::{CostClass, LatencyClass};

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

    #[test]
    fn repeated_matching_is_deterministic_for_equal_candidates() {
        let catalog = vec![
            snapshot("analysis-a", SuggestorRole::Analysis, &[]),
            snapshot("analysis-b", SuggestorRole::Analysis, &[]),
            snapshot("planning-a", SuggestorRole::Planning, &[]),
        ];
        let req = request(
            "r7",
            &[
                SuggestorRole::Analysis,
                SuggestorRole::Analysis,
                SuggestorRole::Planning,
            ],
            &[],
        );

        let first = assemble(&req, &catalog);
        let second = assemble(&req, &catalog);

        assert_eq!(first.assignments, second.assignments);
        assert_eq!(first.unmatched_roles, second.unmatched_roles);
        assert_eq!(first.coverage_ratio, second.coverage_ratio);
    }

    #[tokio::test]
    async fn malformed_request_emits_diagnostic_once() {
        let mut engine = Engine::new();
        engine.register_suggestor(FormationAssemblySuggestor::new(vec![snapshot(
            "analysis-a",
            SuggestorRole::Analysis,
            &[],
        )]));

        let mut ctx = ContextState::new();
        ctx.add_input(ContextKey::Seeds, "formation-request:broken", "{")
            .expect("seed should stage");

        let first = engine.run(ctx).await.expect("run should converge");
        let diagnostics = first.context.get(ContextKey::Diagnostic);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].id,
            "formation-request-error:formation-request:broken"
        );
        assert!(!first.context.has(ContextKey::Strategies));

        let mut rerun_engine = Engine::new();
        rerun_engine.register_suggestor(FormationAssemblySuggestor::new(vec![snapshot(
            "analysis-a",
            SuggestorRole::Analysis,
            &[],
        )]));
        let second = rerun_engine
            .run(first.context.clone())
            .await
            .expect("rerun should converge");
        assert_eq!(second.context.get(ContextKey::Diagnostic).len(), 1);
    }
}
