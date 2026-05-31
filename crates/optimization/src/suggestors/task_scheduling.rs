// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Greedy multi-agent task scheduling with skills and time windows.
//!
//! Reads a [`SchedulingRequest`] from context and proposes a
//! [`SchedulingPlan`] under `ContextKey::Strategies`. This is the portable,
//! pure Rust baseline for the stronger native CP-SAT scheduler in Ferrox.

use async_trait::async_trait;
use converge_pack::Provenance;
use converge_pack::ProvenanceSource;
use converge_pack::{
    AgentEffect, Context, ContextKey, DiagnosticPayload, FactPayload, ProposedFact, Suggestor,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

// -- Request -----------------------------------------------------------------

/// An agent that can execute tasks requiring one of its declared capabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchedulingAgent {
    pub id: usize,
    pub name: String,
    pub capabilities: Vec<String>,
}

/// A unit of work to be scheduled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchedulingTask {
    pub id: usize,
    pub name: String,
    pub required_capability: String,
    pub duration_min: i64,
    pub release_min: i64,
    pub deadline_min: i64,
}

/// Seed under [`ContextKey::Seeds`] with id prefix `"scheduling-request:"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchedulingRequest {
    pub id: String,
    pub agents: Vec<SchedulingAgent>,
    pub tasks: Vec<SchedulingTask>,
    pub horizon_min: i64,
    #[serde(default = "default_time_limit")]
    pub time_limit_seconds: f64,
}

impl FactPayload for SchedulingRequest {
    const FAMILY: &'static str = "converge.optimization.scheduling.request";
    const VERSION: u16 = 1;
}

fn default_time_limit() -> f64 {
    30.0
}

// -- Plan --------------------------------------------------------------------

/// A single task-to-agent assignment with resolved timing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskAssignment {
    pub task_id: usize,
    pub task_name: String,
    pub agent_id: usize,
    pub agent_name: String,
    pub start_min: i64,
    pub end_min: i64,
}

/// Written to [`ContextKey::Strategies`] with id prefix
/// `"scheduling-plan-greedy:"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchedulingPlan {
    pub request_id: String,
    pub assignments: Vec<TaskAssignment>,
    pub tasks_total: usize,
    pub tasks_scheduled: usize,
    pub makespan_min: i64,
    pub solver: String,
    pub status: String,
    pub wall_time_seconds: f64,
}

impl FactPayload for SchedulingPlan {
    const FAMILY: &'static str = "converge.optimization.scheduling.plan";
    const VERSION: u16 = 1;
}

impl SchedulingPlan {
    #[allow(clippy::cast_precision_loss)]
    pub fn throughput_ratio(&self) -> f64 {
        if self.tasks_total == 0 {
            return 0.0;
        }
        self.tasks_scheduled as f64 / self.tasks_total as f64
    }
}

// -- Suggestor ---------------------------------------------------------------

const REQUEST_PREFIX: &str = "scheduling-request:";
const PLAN_PREFIX: &str = "scheduling-plan-greedy:";
const ERROR_PREFIX: &str = "scheduling-request-error:";

/// Schedules tasks via earliest-deadline-first plus earliest-available skilled
/// agent assignment.
pub struct GreedySchedulerSuggestor;

#[async_trait]
impl Suggestor for GreedySchedulerSuggestor {
    fn name(&self) -> &str {
        "GreedySchedulerSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn complexity_hint(&self) -> Option<&'static str> {
        Some("O(n*m*log n) EDF scheduling, n = tasks, m = agents")
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|f| {
            f.id().as_str().starts_with(REQUEST_PREFIX)
                && match f.payload::<SchedulingRequest>() {
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
            match fact.payload::<SchedulingRequest>() {
                Some(req) => {
                    if plan_exists(ctx, req_id(fact.id().as_str())) {
                        continue;
                    }
                    let plan = solve_greedy_schedule(req);
                    let confidence = (plan.throughput_ratio() * 0.65).min(0.65);
                    proposals.push(
                        ProposedFact::new(
                            ContextKey::Strategies,
                            format!("{}{}", PLAN_PREFIX, plan.request_id),
                            plan.clone(),
                            self.name().to_string(),
                        )
                        .with_confidence(confidence),
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
                                    "malformed scheduling request '{}': expected {} v{} payload",
                                    fact.id(),
                                    SchedulingRequest::FAMILY,
                                    SchedulingRequest::VERSION
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

// -- Core logic --------------------------------------------------------------

/// Pure EDF plus earliest-available scheduling. No native solver dependency.
pub fn solve_greedy_schedule(req: &SchedulingRequest) -> SchedulingPlan {
    let t0 = Instant::now();

    let mut ordered: Vec<_> = req.tasks.iter().collect();
    ordered.sort_by_key(|t| (t.deadline_min, t.release_min, t.id));

    let mut next_free = vec![0i64; req.agents.len()];
    let mut assignments = Vec::new();

    for task in &ordered {
        if task.duration_min <= 0 || task.deadline_min < task.release_min + task.duration_min {
            continue;
        }

        let best = req
            .agents
            .iter()
            .enumerate()
            .filter(|(_, agent)| {
                agent
                    .capabilities
                    .iter()
                    .any(|cap| cap == &task.required_capability)
            })
            .map(|(agent_idx, agent)| {
                let start = next_free[agent_idx].max(task.release_min);
                (agent_idx, agent, start)
            })
            .filter(|(_, _, start)| start + task.duration_min <= task.deadline_min)
            .min_by_key(|(_, agent, start)| (*start, agent.id));

        if let Some((agent_idx, agent, start)) = best {
            let end = start + task.duration_min;
            next_free[agent_idx] = end;
            assignments.push(TaskAssignment {
                task_id: task.id,
                task_name: task.name.clone(),
                agent_id: agent.id,
                agent_name: agent.name.clone(),
                start_min: start,
                end_min: end,
            });
        }
    }

    assignments.sort_by_key(|a| (a.start_min, a.agent_id, a.task_id));
    let makespan = assignments.iter().map(|a| a.end_min).max().unwrap_or(0);
    let tasks_scheduled = assignments.len();
    let status = if req.tasks.is_empty() || tasks_scheduled > 0 {
        "feasible"
    } else {
        "infeasible"
    };

    SchedulingPlan {
        request_id: req.id.clone(),
        assignments,
        tasks_total: req.tasks.len(),
        tasks_scheduled,
        makespan_min: makespan,
        solver: "greedy-edf".to_string(),
        status: status.to_string(),
        wall_time_seconds: t0.elapsed().as_secs_f64(),
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::{ContextState, Engine};
    use converge_pack::TextPayload;
    use proptest::prelude::*;
    use std::collections::BTreeMap;

    fn agent(id: usize, name: &str, caps: &[&str]) -> SchedulingAgent {
        SchedulingAgent {
            id,
            name: name.to_string(),
            capabilities: caps.iter().map(|cap| (*cap).to_string()).collect(),
        }
    }

    fn task(id: usize, cap: &str, duration: i64, release: i64, deadline: i64) -> SchedulingTask {
        SchedulingTask {
            id,
            name: format!("task-{id}"),
            required_capability: cap.to_string(),
            duration_min: duration,
            release_min: release,
            deadline_min: deadline,
        }
    }

    fn req(tasks: Vec<SchedulingTask>, agents: Vec<SchedulingAgent>) -> SchedulingRequest {
        SchedulingRequest {
            id: "sched-1".to_string(),
            agents,
            tasks,
            horizon_min: 480,
            time_limit_seconds: 1.0,
        }
    }

    #[tokio::test]
    async fn suggestor_emits_greedy_schedule() {
        let request = req(
            vec![task(1, "rust", 30, 0, 120), task(2, "rust", 30, 0, 120)],
            vec![agent(10, "alice", &["rust"])],
        );

        let mut engine = Engine::new();
        engine.register_suggestor(GreedySchedulerSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_proposal(ProposedFact::new(
            ContextKey::Seeds,
            "scheduling-request:sched-1",
            request,
            "test",
        ))
        .unwrap();

        let result = engine.run(ctx).await.unwrap();
        let facts = result.context.get(ContextKey::Strategies);
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].id().as_str(), "scheduling-plan-greedy:sched-1");
        let plan = facts[0].require_payload::<SchedulingPlan>().unwrap();
        assert_eq!(plan.tasks_scheduled, 2);
        assert_eq!(plan.assignments[0].agent_id, 10);
    }

    #[tokio::test]
    async fn malformed_request_emits_diagnostic() {
        let mut engine = Engine::new();
        engine.register_suggestor(GreedySchedulerSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_proposal(ProposedFact::new(
            ContextKey::Seeds,
            "scheduling-request:bad",
            TextPayload::new("not a scheduling request"),
            "test",
        ))
        .unwrap();

        let result = engine.run(ctx).await.unwrap();
        assert!(result.context.get(ContextKey::Strategies).is_empty());
        assert_eq!(result.context.get(ContextKey::Diagnostic).len(), 1);
    }

    #[test]
    fn non_dense_agent_ids_do_not_index_panic() {
        let plan = solve_greedy_schedule(&req(
            vec![task(1, "rust", 30, 0, 120)],
            vec![agent(99, "alice", &["rust"])],
        ));

        assert_eq!(plan.tasks_scheduled, 1);
        assert_eq!(plan.assignments[0].agent_id, 99);
    }

    proptest! {
        #[test]
        fn scheduled_tasks_respect_windows_and_agent_non_overlap(
            durations in prop::collection::vec(1i64..20, 1..40)
        ) {
            let total_duration: i64 = durations.iter().sum();
            let tasks: Vec<_> = durations
                .iter()
                .enumerate()
                .map(|(i, duration)| task(i, "work", *duration, 0, total_duration + 100))
                .collect();
            let request = req(tasks.clone(), vec![agent(0, "a", &["work"]), agent(1, "b", &["work"])]);
            let plan = solve_greedy_schedule(&request);

            let by_task: BTreeMap<usize, &SchedulingTask> = tasks.iter().map(|t| (t.id, t)).collect();
            let mut by_agent: BTreeMap<usize, Vec<&TaskAssignment>> = BTreeMap::new();

            for assignment in &plan.assignments {
                let original = by_task[&assignment.task_id];
                prop_assert!(assignment.start_min >= original.release_min);
                prop_assert!(assignment.end_min <= original.deadline_min);
                prop_assert_eq!(assignment.end_min - assignment.start_min, original.duration_min);
                by_agent.entry(assignment.agent_id).or_default().push(assignment);
            }

            for assignments in by_agent.values_mut() {
                assignments.sort_by_key(|assignment| assignment.start_min);
                for pair in assignments.windows(2) {
                    prop_assert!(pair[0].end_min <= pair[1].start_min);
                }
            }
        }
    }
}
