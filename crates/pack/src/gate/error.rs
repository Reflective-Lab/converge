//! Error types for gate infrastructure

use thiserror::Error;

pub type GateResult<T> = std::result::Result<T, GateError>;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum GateError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

impl GateError {
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }
}
