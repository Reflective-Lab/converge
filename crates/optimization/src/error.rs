//! Error types for converge-optimization

use thiserror::Error;

/// Result type alias using our Error type
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during optimization
#[derive(Error, Debug, Clone, PartialEq)]
pub enum Error {
    /// Problem has no feasible solution
    #[error("infeasible: {0}")]
    Infeasible(String),

    /// Problem is unbounded (no finite optimal)
    #[error("unbounded: {0}")]
    Unbounded(String),

    /// Invalid input data
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Dimension mismatch in input
    #[error("dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch {
        /// Expected dimension
        expected: usize,
        /// Actual dimension
        got: usize,
    },

    /// Solver timeout
    #[error("timeout after {seconds} seconds")]
    Timeout {
        /// Seconds elapsed before timeout
        seconds: f64,
    },

    /// Numeric overflow during computation
    #[error("numeric overflow: {0}")]
    Overflow(String),

    /// Algorithm did not converge
    #[error("did not converge after {iterations} iterations")]
    NoConvergence {
        /// Iterations completed
        iterations: usize,
    },

    /// Feature requires FFI but it's not enabled
    #[error("FFI feature required for {0}")]
    FfiRequired(String),

    /// Internal error (bug)
    #[error("internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Create an infeasible error
    pub fn infeasible(msg: impl Into<String>) -> Self {
        Self::Infeasible(msg.into())
    }

    /// Create an invalid input error
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }

    /// Create a dimension mismatch error
    pub fn dimension_mismatch(expected: usize, got: usize) -> Self {
        Self::DimensionMismatch { expected, got }
    }

    /// Create a timeout error
    pub fn timeout(seconds: f64) -> Self {
        Self::Timeout { seconds }
    }
}

impl From<converge_pack::GateError> for Error {
    fn from(e: converge_pack::GateError) -> Self {
        match e {
            converge_pack::GateError::InvalidInput(msg) => Self::InvalidInput(msg),
        }
    }
}
