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
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use serde::{Deserialize, Serialize};

use crate::scheduling::{Interval, SchedulingProblem, list_schedule};

// ── Request ───────────────────────────────────────────────────────────────────

/// Seed under [`ContextKey::Seeds`] with id prefix `"schedule-request:"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleRequest {
    pub id: String,
    pub tasks: Vec<ScheduleTask>,
    /// `None` → disjunctive (one machine). `Some(k)` → cumulative (k units).
    pub capacity: Option<i64>,
}

/// A single task to be scheduled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleTask {
    pub label: String,
    pub earliest_start: i64,
    pub latest_end: i64,
    pub duration: i64,
}

// ── Plan (output) ─────────────────────────────────────────────────────────────

/// The schedule produced by the suggestor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulePlan {
    pub request_id: String,
    pub scheduled: Vec<ScheduledTask>,
    pub makespan: i64,
    /// `1.0 - makespan / max_window` — higher means a tighter, more efficient schedule.
    pub efficiency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            f.id.starts_with(REQUEST_PREFIX)
                && match serde_json::from_str::<ScheduleRequest>(&f.content) {
                    Ok(_) => !plan_exists(ctx, req_id(&f.id)),
                    Err(_) => !error_exists(ctx, &f.id),
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
            match serde_json::from_str::<ScheduleRequest>(&fact.content) {
                Ok(req) => {
                    if plan_exists(ctx, req_id(&fact.id)) {
                        continue;
                    }
                    let plan = solve(&req);
                    proposals.push(
                        ProposedFact::new(
                            ContextKey::Strategies,
                            format!("{}{}", PLAN_PREFIX, plan.request_id),
                            serde_json::to_string(&plan).unwrap_or_default(),
                            self.name(),
                        )
                        .with_confidence(plan.efficiency.clamp(0.0, 1.0)),
                    );
                }
                Err(e) => {
                    if error_exists(ctx, &fact.id) {
                        continue;
                    }
                    let diag = serde_json::json!({
                        "request_fact_id": fact.id,
                        "message": "malformed schedule request",
                        "error": e.to_string(),
                    });
                    proposals.push(
                        ProposedFact::new(
                            ContextKey::Diagnostic,
                            format!("{}{}", ERROR_PREFIX, fact.id),
                            diag.to_string(),
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
    ctx.get(ContextKey::Strategies).iter().any(|f| f.id == id)
}

fn error_exists(ctx: &dyn Context, fact_id: &str) -> bool {
    let id = format!("{}{}", ERROR_PREFIX, fact_id);
    ctx.get(ContextKey::Diagnostic).iter().any(|f| f.id == id)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::{ContextState, Engine};

    fn req_json(id: &str, tasks: Vec<(&str, i64, i64, i64)>, capacity: Option<i64>) -> String {
        serde_json::to_string(&ScheduleRequest {
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
        })
        .unwrap()
    }

    #[tokio::test]
    async fn three_tasks_makespan_equals_sum() {
        let mut engine = Engine::new();
        engine.register_suggestor(WorkScheduleSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_input(
            ContextKey::Seeds,
            "schedule-request:r1",
            req_json(
                "r1",
                vec![
                    ("design", 0, 30, 5),
                    ("build", 0, 30, 8),
                    ("test", 0, 30, 3),
                ],
                None,
            ),
        )
        .unwrap();

        let result = engine.run(ctx).await.unwrap();
        let facts = result.context.get(ContextKey::Strategies);
        assert_eq!(facts.len(), 1);
        let plan: SchedulePlan = serde_json::from_str(&facts[0].content).unwrap();
        assert_eq!(plan.makespan, 16, "3 sequential tasks: 5+8+3=16");
        assert_eq!(plan.scheduled.len(), 3);
    }

    #[tokio::test]
    async fn result_is_idempotent() {
        let mut engine = Engine::new();
        engine.register_suggestor(WorkScheduleSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_input(
            ContextKey::Seeds,
            "schedule-request:r1",
            req_json("r1", vec![("a", 0, 20, 5), ("b", 0, 20, 3)], None),
        )
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
        ctx.add_input(ContextKey::Seeds, "schedule-request:bad", "not-json")
            .unwrap();

        let result = engine.run(ctx).await.unwrap();
        assert_eq!(result.context.get(ContextKey::Diagnostic).len(), 1);
        assert!(!result.context.has(ContextKey::Strategies));
    }
}
