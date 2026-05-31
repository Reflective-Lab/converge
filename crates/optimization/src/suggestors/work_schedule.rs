// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Work scheduling via the list-scheduling heuristic.
//!
//! Reads a [`ScheduleRequest`] from context and proposes a [`SchedulePlan`]
//! that assigns each task a concrete start time with no resource conflicts.
//!
//! # Formation role
//!
//! An assignment suggestor settles who does what; this suggestor settles when.
//! If upstream capacity estimates change, a new request is seeded and the
//! formation re-converges on an updated schedule.
//!
//! # Modes
//!
//! - **Disjunctive** (`capacity: None`) — single machine, tasks cannot overlap.
//! - **Cumulative** (`capacity: Some(k)`) — up to k units of resource may be
//!   active simultaneously (parallel machines / multi-resource).

use async_trait::async_trait;
use converge_pack::Provenance;
use converge_pack::ProvenanceSource;
use converge_pack::{
    AgentEffect, Context, ContextKey, DiagnosticPayload, FactPayload, ProposedFact, Suggestor,
};
use serde::{Deserialize, Serialize};

use crate::scheduling::{Interval, SchedulingProblem, list_schedule};

// ── Request ───────────────────────────────────────────────────────────────────

/// Seed under [`ContextKey::Seeds`] with id prefix `"schedule-request:"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleRequest {
    pub id: String,
    pub tasks: Vec<ScheduleTask>,
    /// `None` → disjunctive (one machine). `Some(k)` → cumulative (k units).
    pub capacity: Option<i64>,
}

impl FactPayload for ScheduleRequest {
    const FAMILY: &'static str = "converge.optimization.schedule.request";
    const VERSION: u16 = 1;
}

/// A single task to be scheduled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleTask {
    pub label: String,
    pub earliest_start: i64,
    pub latest_end: i64,
    pub duration: i64,
}

// ── Plan (output) ─────────────────────────────────────────────────────────────

/// The schedule produced by the suggestor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchedulePlan {
    pub request_id: String,
    pub scheduled: Vec<ScheduledTask>,
    pub makespan: i64,
    /// `1.0 - makespan / max_window` — higher means a tighter, more efficient schedule.
    pub efficiency: f64,
}

impl FactPayload for SchedulePlan {
    const FAMILY: &'static str = "converge.optimization.schedule.plan";
    const VERSION: u16 = 1;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduledTask {
    pub label: String,
    pub start: i64,
    pub end: i64,
}

// ── Suggestor ─────────────────────────────────────────────────────────────────

const REQUEST_PREFIX: &str = "schedule-request:";
const PLAN_PREFIX: &str = "schedule-plan:";
const ERROR_PREFIX: &str = "schedule-request-error:";

/// Schedules tasks respecting time windows and resource capacity constraints.
pub struct WorkScheduleSuggestor;

#[async_trait]
impl Suggestor for WorkScheduleSuggestor {
    fn name(&self) -> &str {
        "WorkScheduleSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn complexity_hint(&self) -> Option<&'static str> {
        Some("O(n log n) list scheduling — n = tasks; scales to thousands of tasks")
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|f| {
            f.id().as_str().starts_with(REQUEST_PREFIX)
                && match f.payload::<ScheduleRequest>() {
                    Some(_) => !plan_exists(ctx, req_id(f.id().as_str())),
                    None => !error_exists(ctx, f.id().as_str()),
                }
        })
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut proposals = Vec::new();

        for fact in ctx
            .get(ContextKey::Seeds)
            .iter()
            .filter(|f| f.id().as_str().starts_with(REQUEST_PREFIX))
        {
            match fact.payload::<ScheduleRequest>() {
                Some(req) => {
                    if plan_exists(ctx, req_id(fact.id().as_str())) {
                        continue;
                    }
                    let plan = solve(req);
                    proposals.push(
                        ProposedFact::new(
                            ContextKey::Strategies,
                            format!("{}{}", PLAN_PREFIX, plan.request_id),
                            plan.clone(),
                            self.name().to_string(),
                        )
                        .with_confidence(plan.efficiency.clamp(0.0, 1.0)),
                    );
                }
                None => {
                    if error_exists(ctx, fact.id().as_str()) {
                        continue;
                    }
                    proposals.push(
                        ProposedFact::new(
                            ContextKey::Diagnostic,
                            format!("{}{}", ERROR_PREFIX, fact.id()),
                            DiagnosticPayload::new(
                                self.name(),
                                format!(
                                    "malformed schedule request '{}': expected {} v{} payload",
                                    fact.id(),
                                    ScheduleRequest::FAMILY,
                                    ScheduleRequest::VERSION
                                ),
                            ),
                            self.name().to_string(),
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

    fn provenance(&self) -> Provenance {
        Provenance::from(super::CONVERGE_OPTIMIZATION_PROVENANCE.as_str())
    }
}

// ── Core logic ────────────────────────────────────────────────────────────────

fn solve(req: &ScheduleRequest) -> SchedulePlan {
    if req.tasks.is_empty() {
        return SchedulePlan {
            request_id: req.id.clone(),
            scheduled: vec![],
            makespan: 0,
            efficiency: 1.0,
        };
    }

    let intervals: Vec<Interval> = req
        .tasks
        .iter()
        .enumerate()
        .map(|(i, t)| Interval::new(i, t.earliest_start, t.latest_end, t.duration))
        .collect();

    let max_window = req.tasks.iter().map(|t| t.latest_end).max().unwrap_or(1);

    let problem = match req.capacity {
        None => SchedulingProblem::disjunctive(intervals),
        Some(cap) => SchedulingProblem::cumulative(intervals, cap),
    };

    match list_schedule(&problem) {
        Ok(sol) => {
            let scheduled = sol
                .schedule
                .iter()
                .map(|s| ScheduledTask {
                    label: req
                        .tasks
                        .get(s.interval.id)
                        .map(|t| t.label.clone())
                        .unwrap_or_default(),
                    start: s.start,
                    end: s.end(),
                })
                .collect();
            let efficiency = if max_window > 0 {
                1.0 - sol.makespan as f64 / max_window as f64
            } else {
                1.0
            };
            SchedulePlan {
                request_id: req.id.clone(),
                scheduled,
                makespan: sol.makespan,
                efficiency: efficiency.clamp(0.0, 1.0),
            }
        }
        Err(_) => {
            // Infeasible schedules surface as a low-confidence plan with
            // makespan = -1 so downstream suggestors can detect the failure.
            SchedulePlan {
                request_id: req.id.clone(),
                scheduled: vec![],
                makespan: -1,
                efficiency: 0.0,
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn req_id(fact_id: &str) -> &str {
    fact_id.trim_start_matches(REQUEST_PREFIX)
}

fn plan_exists(ctx: &dyn Context, request_id: &str) -> bool {
    let id = format!("{}{}", PLAN_PREFIX, request_id);
    ctx.get(ContextKey::Strategies)
        .iter()
        .any(|f| f.id().as_str() == id)
}

fn error_exists(ctx: &dyn Context, fact_id: &str) -> bool {
    let id = format!("{}{}", ERROR_PREFIX, fact_id);
    ctx.get(ContextKey::Diagnostic)
        .iter()
        .any(|f| f.id().as_str() == id)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::{ContextState, Engine};
    use converge_pack::TextPayload;

    fn req(id: &str, tasks: Vec<(&str, i64, i64, i64)>, capacity: Option<i64>) -> ScheduleRequest {
        ScheduleRequest {
            id: id.to_string(),
            tasks: tasks
                .into_iter()
                .map(|(label, es, le, dur)| ScheduleTask {
                    label: label.to_string(),
                    earliest_start: es,
                    latest_end: le,
                    duration: dur,
                })
                .collect(),
            capacity,
        }
    }

    #[tokio::test]
    async fn three_tasks_makespan_equals_sum() {
        let mut engine = Engine::new();
        engine.register_suggestor(WorkScheduleSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_proposal(ProposedFact::new(
            ContextKey::Seeds,
            "schedule-request:r1",
            req(
                "r1",
                vec![
                    ("design", 0, 30, 5),
                    ("build", 0, 30, 8),
                    ("test", 0, 30, 3),
                ],
                None,
            ),
            "test",
        ))
        .unwrap();

        let result = engine.run(ctx).await.unwrap();
        let facts = result.context.get(ContextKey::Strategies);
        assert_eq!(facts.len(), 1);
        let plan = facts[0].require_payload::<SchedulePlan>().unwrap();
        assert_eq!(plan.makespan, 16, "3 sequential tasks: 5+8+3=16");
        assert_eq!(plan.scheduled.len(), 3);
    }

    #[tokio::test]
    async fn result_is_idempotent() {
        let mut engine = Engine::new();
        engine.register_suggestor(WorkScheduleSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_proposal(ProposedFact::new(
            ContextKey::Seeds,
            "schedule-request:r1",
            req("r1", vec![("a", 0, 20, 5), ("b", 0, 20, 3)], None),
            "test",
        ))
        .unwrap();

        let first = engine.run(ctx).await.unwrap();
        let mut engine2 = Engine::new();
        engine2.register_suggestor(WorkScheduleSuggestor);
        let second = engine2.run(first.context.clone()).await.unwrap();
        assert_eq!(
            second.context.get(ContextKey::Strategies).len(),
            first.context.get(ContextKey::Strategies).len(),
        );
    }

    #[tokio::test]
    async fn malformed_request_emits_diagnostic() {
        let mut engine = Engine::new();
        engine.register_suggestor(WorkScheduleSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_proposal(ProposedFact::new(
            ContextKey::Seeds,
            "schedule-request:bad",
            TextPayload::new("not a schedule request"),
            "test",
        ))
        .unwrap();

        let result = engine.run(ctx).await.unwrap();
        assert_eq!(result.context.get(ContextKey::Diagnostic).len(), 1);
        assert!(!result.context.has(ContextKey::Strategies));
    }
}
