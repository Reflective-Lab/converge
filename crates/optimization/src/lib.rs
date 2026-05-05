//! # converge-optimization
//!
//! Optimization solvers as first-class Suggestors for the Converge Engine.
//!
//! Every solver is accessed through [`PackSuggestor`] -- the ONLY public
//! interface. Register it in a formation and let it converge alongside
//! LLM agents, policy gates, and other Suggestors.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use converge_pack::{PackSuggestor, ContextKey};
//! use converge_optimization::packs::budget_allocation::BudgetAllocationPack;
//!
//! let solver = PackSuggestor::new(
//!     BudgetAllocationPack,
//!     ContextKey::Seeds,
//!     ContextKey::Strategies,
//! );
//! engine.register_suggestor(solver);
//! ```
//!
//! ## Available Packs (21)
//!
//! LeadRouting, MeetingScheduler, BudgetAllocation, CapacityPlanning,
//! InventoryReplenishment, InventoryRebalancing, AnomalyTriage,
//! PricingGuardrails, ShippingChoice, VendorShortlist, BacklogPrioritization,
//! AssignmentPack, BinPacking, ConstraintProgramming, FacilityLocation,
//! GraphPartitioning, JobShopScheduling, NetworkFlow, StaffRostering,
//! TravelingSalesman, VehicleRouting
//!
//! ## Feature Flags
//!
//! - `sat` - Varisat SAT solver for constraint programming
//! - `full` - All native optimization features

// ── Public API: Pack types re-exported from converge-pack ────────────

pub mod packs;

pub use converge_pack::{Pack, PackSolveResult, PackSuggestor};

/// Extension trait for `SolveBudgets` that bridges to optimization-specific `SolverParams`.
pub trait SolveBudgetsExt {
    /// Convert to SolverParams for existing solvers
    fn to_solver_params(&self, seed: u64) -> SolverParams;
}

impl SolveBudgetsExt for converge_pack::SolveBudgets {
    fn to_solver_params(&self, seed: u64) -> SolverParams {
        SolverParams {
            time_limit_seconds: self.time_limit.as_secs_f64(),
            iteration_limit: self.iteration_limit,
            num_threads: 0,
            random_seed: seed,
            verbosity: 0,
        }
    }
}

// ── Algorithm implementations (used by Packs) ────────────────────────

pub mod assignment;
pub mod graph;
pub mod knapsack;
pub mod provider;
pub mod scheduling;
pub mod setcover;
pub mod suggestors;

#[cfg(feature = "sat")]
pub mod cp;

mod error;
mod types;

pub use error::{Error, Result};
pub use types::*;
