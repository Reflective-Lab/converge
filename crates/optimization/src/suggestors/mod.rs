// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

use converge_pack::ProvenanceSource;

/// Canonical provenance marker for `converge-optimization` fact-emitting
/// suggestors. All in-tree suggestors in this crate share this provenance
/// string; downstream extensions that build on `converge-optimization`
/// declare their own `ProvenanceSource`.
#[derive(Copy, Clone, Debug)]
pub struct ConvergeOptimization;

impl ProvenanceSource for ConvergeOptimization {
    fn as_str(&self) -> &'static str {
        "converge-optimization"
    }
}

/// Canonical provenance const for [`ConvergeOptimization`].
pub const CONVERGE_OPTIMIZATION_PROVENANCE: ConvergeOptimization = ConvergeOptimization;

pub mod assignment;
pub mod flow_optimization;
pub mod formation;
pub mod portfolio;
pub mod task_scheduling;
pub mod time_window_routing;
pub mod work_schedule;

pub use assignment::{AssignmentPlan, AssignmentRequest, AssignmentSuggestor};
pub use flow_optimization::{FlowEdgeSpec, FlowOptimizationSuggestor, FlowPlan, FlowRequest};
pub use formation::FormationAssemblySuggestor;
pub use portfolio::{PortfolioItem, PortfolioRequest, PortfolioSelection, PortfolioSuggestor};
pub use task_scheduling::{
    GreedySchedulerSuggestor, SchedulingAgent, SchedulingPlan, SchedulingRequest, SchedulingTask,
    TaskAssignment,
};
pub use time_window_routing::{
    NearestNeighborTimeWindowRoutingSuggestor, RouteStop, VrptwCustomer, VrptwDepot, VrptwPlan,
    VrptwRequest,
};
pub use work_schedule::{
    SchedulePlan, ScheduleRequest, ScheduleTask, ScheduledTask, WorkScheduleSuggestor,
};
