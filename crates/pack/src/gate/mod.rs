//! Solver Gate Architecture
//!
//! This module provides types for treating optimization as a governed,
//! deterministic gate in enterprise workflows.
//!
//! ## Core Concepts
//!
//! - **ProblemSpec**: Immutable input with tenant scope, budgets, and provenance
//! - **ProposedPlan**: Output plan with confidence scoring and trace links
//! - **SolverReport**: Detailed solver execution report for audit
//! - **PromotionGate**: Decision framework for plan approval
//!
//! ## Flow
//!
//! ```text
//! ProblemSpec -> ProposedPlan -> SolverReport -> PromotionGate
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use converge_pack::gate::*;
//!
//! let spec = ProblemSpec::builder("prob-001", "tenant-abc")
//!     .objective(ObjectiveSpec::minimize("cost"))
//!     .budgets(SolveBudgets::with_time_limit(30))
//!     .build()?;
//! ```

pub mod budgets;
pub mod constraints;
pub mod decision;
pub mod determinism;
mod error;
pub mod provenance;
pub mod report;
pub mod types;

pub use budgets::*;
pub use constraints::*;
pub use decision::*;
pub use determinism::*;
pub use error::{GateError, GateResult};
pub use provenance::*;
pub use report::*;
pub use types::*;
