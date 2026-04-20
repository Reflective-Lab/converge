//! # converge-optimization
//!
//! Optimization solvers as first-class Suggestors for the Converge Engine.
//!
//! Every solver is accessed through [`SolverSuggestor`] — the ONLY public
//! interface. Register it in a formation and let it converge alongside
//! LLM agents, policy gates, and other Suggestors.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use converge_optimization::{SolverSuggestor, packs::budget_allocation::BudgetAllocationPack};
//! use converge_pack::ContextKey;
//!
//! let solver = SolverSuggestor::new(
//!     BudgetAllocationPack,
//!     ContextKey::Seeds,
//!     ContextKey::Strategies,
//! );
//! engine.register_suggestor(solver);
//! ```
//!
//! ## Available Packs (11)
//!
//! LeadRouting, MeetingScheduler, BudgetAllocation, CapacityPlanning,
//! InventoryReplenishment, InventoryRebalancing, AnomalyTriage,
//! PricingGuardrails, ShippingChoice, VendorShortlist, BacklogPrioritization
//!
//! ## Feature Flags
//!
//! - `sat` - Varisat SAT solver for constraint programming
//! - `ffi` - OR-Tools C++ FFI bindings
//! - `full` - All features

// ── Public API: Suggestor interface only ──────────────────────────────

pub mod packs;
pub mod suggestor;

pub use packs::{Pack, PackRegistry, PackSolveResult};
pub use suggestor::SolverSuggestor;

// ── Algorithm implementations (used by Packs) ────────────────────────

pub mod assignment;
pub mod gate;
pub mod graph;
pub mod knapsack;
pub mod provider;
pub mod scheduling;
pub mod setcover;

#[cfg(feature = "sat")]
pub mod cp;

mod error;
mod types;

pub use error::{Error, Result};
pub use types::*;
