//! Domain Packs for Solver Gate
//!
//! Each pack provides:
//! - Typed input/output schemas
//! - Domain-specific solvers
//! - Invariant definitions
//! - Test scenarios
//!
//! ## Available Packs
//!
//! - [`meeting_scheduler`] - Meeting time selection with preferences
//! - [`inventory_rebalancing`] - Inventory transfer planning
//!
//! ## Example
//!
//! ```rust,ignore
//! use converge_optimization::packs::{PackRegistry, Pack};
//! use converge_optimization::gate::ProblemSpec;
//!
//! let registry = PackRegistry::with_builtins();
//! let pack = registry.get("meeting-scheduler").unwrap();
//! let result = pack.solve(&spec)?;
//! ```

pub mod registry;
pub mod testing;
pub mod traits;

// Fully implemented packs
pub mod inventory_rebalancing;
pub mod meeting_scheduler;

// Stub packs (types + placeholder solver)
pub mod anomaly_triage;
pub mod assignment_pack;
pub mod backlog_prioritization;
pub mod bin_packing;
pub mod budget_allocation;
pub mod capacity_planning;
pub mod facility_location;
pub mod inventory_replenishment;
pub mod lead_routing;
pub mod network_flow;
pub mod pricing_guardrails;
pub mod shipping_choice;
pub mod vehicle_routing;
pub mod vendor_shortlist;

pub use registry::*;
pub use testing::{ExpectedOutcome, ScenarioResult, TestScenario};
pub use traits::*;
