//! Resource budgets for solver execution

use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::error::{GateError, GateResult};

/// Resource budgets for solver execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveBudgets {
    /// Maximum wall-clock time for solving (in seconds)
    #[serde(with = "duration_serde")]
    pub time_limit: Duration,
    /// Maximum solver iterations
    pub iteration_limit: usize,
    /// Maximum candidate solutions to evaluate
    pub candidate_cap: usize,
    /// Maximum memory usage (bytes, 0 = unlimited)
    pub memory_limit_bytes: usize,
}

impl Default for SolveBudgets {
    fn default() -> Self {
        Self {
            time_limit: Duration::from_secs(30),
            iteration_limit: 100_000,
            candidate_cap: 1_000,
            memory_limit_bytes: 0,
        }
    }
}

impl SolveBudgets {
    /// Create budgets with time limit only
    pub fn with_time_limit(seconds: u64) -> Self {
        Self {
            time_limit: Duration::from_secs(seconds),
            ..Default::default()
        }
    }

    /// Create strict budgets for testing
    pub fn strict(time_seconds: u64, iterations: usize, candidates: usize) -> Self {
        Self {
            time_limit: Duration::from_secs(time_seconds),
            iteration_limit: iterations,
            candidate_cap: candidates,
            memory_limit_bytes: 0,
        }
    }

    /// Validate budgets are reasonable
    pub fn validate(&self) -> GateResult<()> {
        if self.time_limit.is_zero() {
            return Err(GateError::invalid_input("time_limit must be positive"));
        }
        if self.iteration_limit == 0 {
            return Err(GateError::invalid_input("iteration_limit must be positive"));
        }
        if self.candidate_cap == 0 {
            return Err(GateError::invalid_input("candidate_cap must be positive"));
        }
        Ok(())
    }

    /// Check if time budget allows more work
    pub fn has_time_remaining(&self, elapsed: Duration) -> bool {
        elapsed < self.time_limit
    }
}

/// Serde support for Duration as seconds
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs_f64().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = f64::deserialize(deserializer)?;
        Ok(Duration::from_secs_f64(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_budgets() {
        let budgets = SolveBudgets::default();
        assert_eq!(budgets.time_limit, Duration::from_secs(30));
        assert_eq!(budgets.iteration_limit, 100_000);
        assert_eq!(budgets.candidate_cap, 1_000);
    }

    #[test]
    fn test_validate_budgets() {
        let budgets = SolveBudgets::default();
        assert!(budgets.validate().is_ok());

        let invalid = SolveBudgets {
            time_limit: Duration::ZERO,
            ..Default::default()
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_serde_roundtrip() {
        let budgets = SolveBudgets::with_time_limit(45);
        let json = serde_json::to_string(&budgets).unwrap();
        let restored: SolveBudgets = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.time_limit, budgets.time_limit);
    }
}
