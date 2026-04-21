//! Constraint and objective specifications

use serde::{Deserialize, Serialize};

/// Objective specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveSpec {
    /// Objective direction
    pub direction: ObjectiveDirection,
    /// Name of the metric to optimize
    pub metric: String,
    /// Weight for multi-objective (1.0 for single)
    pub weight: f64,
}

impl ObjectiveSpec {
    /// Create minimize objective
    pub fn minimize(metric: impl Into<String>) -> Self {
        Self {
            direction: ObjectiveDirection::Minimize,
            metric: metric.into(),
            weight: 1.0,
        }
    }

    /// Create maximize objective
    pub fn maximize(metric: impl Into<String>) -> Self {
        Self {
            direction: ObjectiveDirection::Maximize,
            metric: metric.into(),
            weight: 1.0,
        }
    }

    /// Set weight for multi-objective optimization
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// Check if this is a minimization objective
    pub fn is_minimize(&self) -> bool {
        self.direction == ObjectiveDirection::Minimize
    }

    /// Compare two values according to objective direction
    /// Returns true if `a` is better than `b`
    pub fn is_better(&self, a: f64, b: f64) -> bool {
        match self.direction {
            ObjectiveDirection::Minimize => a < b,
            ObjectiveDirection::Maximize => a > b,
        }
    }
}

/// Objective direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveDirection {
    /// Minimize the objective
    Minimize,
    /// Maximize the objective
    Maximize,
}

/// Constraint specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintSpec {
    /// Constraint name
    pub name: String,
    /// Constraint type
    pub constraint_type: ConstraintType,
    /// Whether this is a hard (must satisfy) or soft constraint
    pub hardness: ConstraintHardness,
    /// Penalty weight for soft constraints
    pub penalty_weight: f64,
}

impl ConstraintSpec {
    /// Create a hard constraint
    pub fn hard(name: impl Into<String>, constraint_type: ConstraintType) -> Self {
        Self {
            name: name.into(),
            constraint_type,
            hardness: ConstraintHardness::Hard,
            penalty_weight: 0.0,
        }
    }

    /// Create a soft constraint with penalty
    pub fn soft(name: impl Into<String>, constraint_type: ConstraintType, penalty: f64) -> Self {
        Self {
            name: name.into(),
            constraint_type,
            hardness: ConstraintHardness::Soft,
            penalty_weight: penalty,
        }
    }

    /// Check if this is a hard constraint
    pub fn is_hard(&self) -> bool {
        self.hardness == ConstraintHardness::Hard
    }
}

/// Constraint type (pack-specific interpretation)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConstraintType {
    /// Capacity constraint
    Capacity {
        /// Resource being constrained
        resource: String,
        /// Maximum limit
        limit: f64,
    },
    /// Time window constraint
    TimeWindow {
        /// Start time (unix timestamp)
        start: i64,
        /// End time (unix timestamp)
        end: i64,
    },
    /// Precedence constraint
    Precedence {
        /// Item that must come before
        before: String,
        /// Item that must come after
        after: String,
    },
    /// Exclusion constraint (mutual exclusivity)
    Exclusion {
        /// Items that cannot be selected together
        items: Vec<String>,
    },
    /// Minimum requirement
    Minimum {
        /// Resource or metric
        resource: String,
        /// Minimum value required
        value: f64,
    },
    /// Maximum limit
    Maximum {
        /// Resource or metric
        resource: String,
        /// Maximum value allowed
        value: f64,
    },
    /// Custom constraint (pack interprets)
    Custom {
        /// Constraint key
        key: String,
        /// Constraint value
        value: serde_json::Value,
    },
}

impl ConstraintType {
    /// Create a capacity constraint
    pub fn capacity(resource: impl Into<String>, limit: f64) -> Self {
        Self::Capacity {
            resource: resource.into(),
            limit,
        }
    }

    /// Create a time window constraint
    pub fn time_window(start: i64, end: i64) -> Self {
        Self::TimeWindow { start, end }
    }

    /// Create a precedence constraint
    pub fn precedence(before: impl Into<String>, after: impl Into<String>) -> Self {
        Self::Precedence {
            before: before.into(),
            after: after.into(),
        }
    }

    /// Create an exclusion constraint
    pub fn exclusion(items: Vec<String>) -> Self {
        Self::Exclusion { items }
    }
}

/// Constraint hardness
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintHardness {
    /// Must be satisfied
    Hard,
    /// Can be violated with penalty
    Soft,
}

/// A constraint violation in solution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// Which constraint was violated
    pub constraint_name: String,
    /// Severity (0.0 = marginal, 1.0 = complete violation)
    pub severity: f64,
    /// Human-readable explanation
    pub explanation: String,
    /// Affected entities
    pub affected_entities: Vec<String>,
}

impl Violation {
    /// Create a new violation
    pub fn new(
        constraint_name: impl Into<String>,
        severity: f64,
        explanation: impl Into<String>,
    ) -> Self {
        Self {
            constraint_name: constraint_name.into(),
            severity: severity.clamp(0.0, 1.0),
            explanation: explanation.into(),
            affected_entities: Vec::new(),
        }
    }

    /// Add affected entity
    pub fn with_affected(mut self, entity: impl Into<String>) -> Self {
        self.affected_entities.push(entity.into());
        self
    }

    /// Add multiple affected entities
    pub fn with_affected_all(
        mut self,
        entities: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        for e in entities {
            self.affected_entities.push(e.into());
        }
        self
    }

    /// Check if this is a severe violation
    pub fn is_severe(&self) -> bool {
        self.severity >= 0.8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_objective_minimize() {
        let obj = ObjectiveSpec::minimize("cost");
        assert!(obj.is_minimize());
        assert!(obj.is_better(10.0, 20.0));
    }

    #[test]
    fn test_objective_maximize() {
        let obj = ObjectiveSpec::maximize("profit");
        assert!(!obj.is_minimize());
        assert!(obj.is_better(20.0, 10.0));
    }

    #[test]
    fn test_constraint_hard() {
        let c = ConstraintSpec::hard("capacity", ConstraintType::capacity("memory", 1024.0));
        assert!(c.is_hard());
        assert_eq!(c.penalty_weight, 0.0);
    }

    #[test]
    fn test_constraint_soft() {
        let c = ConstraintSpec::soft("preference", ConstraintType::time_window(0, 100), 0.5);
        assert!(!c.is_hard());
        assert_eq!(c.penalty_weight, 0.5);
    }

    #[test]
    fn test_violation() {
        let v = Violation::new("capacity", 0.9, "exceeded by 10%")
            .with_affected("node-1")
            .with_affected("node-2");

        assert!(v.is_severe());
        assert_eq!(v.affected_entities.len(), 2);
    }

    #[test]
    fn test_severity_clamped() {
        let v = Violation::new("test", 1.5, "over max");
        assert_eq!(v.severity, 1.0);

        let v2 = Violation::new("test", -0.5, "under min");
        assert_eq!(v2.severity, 0.0);
    }

    #[test]
    fn test_constraint_serde() {
        let c = ConstraintSpec::hard("cap", ConstraintType::capacity("cpu", 100.0));
        let json = serde_json::to_string(&c).unwrap();
        let restored: ConstraintSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "cap");
    }
}
