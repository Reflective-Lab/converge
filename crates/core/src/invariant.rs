// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Invariant system for Converge.
//!
//! Invariants are runtime constraints that the engine enforces.
//! They are compiled from Gherkin specs into Rust predicates.
//!
//! # Invariant Classes
//!
//! - **Structural**: Checked on every merge. Violation = immediate failure.
//! - **Semantic**: Checked per cycle. Violation = blocks convergence.
//! - **Acceptance**: Checked at convergence. Violation = rejects results.
//!
//! # Example
//!
//! ```
//! use converge_core::invariant::{Invariant, InvariantClass, InvariantResult};
//! use converge_core::Context;
//!
//! struct NoEmptyFacts;
//!
//! impl Invariant for NoEmptyFacts {
//!     fn name(&self) -> &str { "no_empty_facts" }
//!     fn class(&self) -> InvariantClass { InvariantClass::Structural }
//!
//!     fn check(&self, ctx: &dyn Context) -> InvariantResult {
//!         // Check logic here
//!         InvariantResult::Ok
//!     }
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::context::{ContextState, FactId};

/// The class of an invariant determines when it's checked and how violations are handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InvariantClass {
    /// Checked on every merge. Violation causes immediate failure.
    /// Examples: schema validity, type correctness, forbidden combinations.
    Structural,

    /// Checked at the end of each cycle. Violation blocks convergence.
    /// Examples: "no strategy violates brand safety".
    Semantic,

    /// Checked when convergence is claimed. Violation rejects results.
    /// Examples: "at least two viable strategies exist".
    Acceptance,
}

/// The result of checking an invariant.
#[derive(Debug, Clone, PartialEq)]
pub enum InvariantResult {
    /// Invariant holds.
    Ok,
    /// Invariant is violated.
    Violated(Violation),
}

impl InvariantResult {
    /// Returns true if the invariant holds.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok)
    }

    /// Returns true if the invariant is violated.
    #[must_use]
    pub fn is_violated(&self) -> bool {
        matches!(self, Self::Violated(_))
    }
}

/// Details of an invariant violation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Violation {
    /// Human-readable description of what went wrong.
    pub reason: String,
    /// Optional: which facts contributed to the violation.
    pub fact_ids: Vec<FactId>,
}

impl Violation {
    /// Creates a new violation with just a reason.
    #[must_use]
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            fact_ids: Vec::new(),
        }
    }

    /// Creates a violation with associated fact IDs.
    #[must_use]
    pub fn with_facts(reason: impl Into<String>, fact_ids: Vec<FactId>) -> Self {
        Self {
            reason: reason.into(),
            fact_ids,
        }
    }
}

/// A runtime invariant that the engine enforces.
///
/// Invariants are the "law" that Gherkin specs compile to.
pub trait Invariant: Send + Sync {
    /// Human-readable name for tracing and error messages.
    fn name(&self) -> &str;

    /// The class determines when this invariant is checked.
    fn class(&self) -> InvariantClass;

    /// Check the invariant against the current context.
    fn check(&self, ctx: &dyn crate::Context) -> InvariantResult;
}

/// Unique identifier for a registered invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InvariantId(pub(crate) u32);

impl std::fmt::Display for InvariantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invariant({})", self.0)
    }
}

/// Registry of invariants, organized by class for efficient checking.
#[derive(Default)]
pub struct InvariantRegistry {
    invariants: Vec<Box<dyn Invariant>>,
    by_class: HashMap<InvariantClass, Vec<InvariantId>>,
    next_id: u32,
}

impl InvariantRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an invariant and returns its ID.
    pub fn register(&mut self, invariant: impl Invariant + 'static) -> InvariantId {
        let id = InvariantId(self.next_id);
        self.next_id += 1;

        let class = invariant.class();
        self.by_class.entry(class).or_default().push(id);
        self.invariants.push(Box::new(invariant));

        id
    }

    /// Returns the number of registered invariants.
    #[must_use]
    pub fn count(&self) -> usize {
        self.invariants.len()
    }

    /// Checks all invariants of a given class.
    ///
    /// Returns the first violation found, or Ok if all pass.
    ///
    /// # Errors
    ///
    /// Returns `InvariantError` if any invariant of the given class is violated.
    pub fn check_class(
        &self,
        class: InvariantClass,
        ctx: &ContextState,
    ) -> Result<(), InvariantError> {
        let ids = self.by_class.get(&class).map_or(&[][..], Vec::as_slice);

        for &id in ids {
            let invariant = &self.invariants[id.0 as usize];
            if let InvariantResult::Violated(violation) = invariant.check(ctx) {
                return Err(InvariantError {
                    invariant_name: invariant.name().to_string(),
                    class,
                    violation,
                });
            }
        }

        Ok(())
    }

    /// Checks all structural invariants.
    ///
    /// # Errors
    ///
    /// Returns `InvariantError` if any structural invariant is violated.
    pub fn check_structural(&self, ctx: &ContextState) -> Result<(), InvariantError> {
        self.check_class(InvariantClass::Structural, ctx)
    }

    /// Checks all semantic invariants.
    ///
    /// # Errors
    ///
    /// Returns `InvariantError` if any semantic invariant is violated.
    pub fn check_semantic(&self, ctx: &ContextState) -> Result<(), InvariantError> {
        self.check_class(InvariantClass::Semantic, ctx)
    }

    /// Checks all acceptance invariants.
    ///
    /// # Errors
    ///
    /// Returns `InvariantError` if any acceptance invariant is violated.
    pub fn check_acceptance(&self, ctx: &ContextState) -> Result<(), InvariantError> {
        self.check_class(InvariantClass::Acceptance, ctx)
    }
}

/// Error returned when an invariant is violated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantError {
    /// Name of the invariant that was violated.
    pub invariant_name: String,
    /// Class of the invariant.
    pub class: InvariantClass,
    /// Details of the violation.
    pub violation: Violation,
}

impl std::fmt::Display for InvariantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} invariant '{}' violated: {}",
            self.class, self.invariant_name, self.violation.reason
        )
    }
}

impl std::error::Error for InvariantError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextKey;

    /// Invariant that requires at least one seed.
    struct RequireSeeds;

    impl Invariant for RequireSeeds {
        fn name(&self) -> &'static str {
            "require_seeds"
        }

        fn class(&self) -> InvariantClass {
            InvariantClass::Acceptance
        }

        fn check(&self, ctx: &dyn crate::Context) -> InvariantResult {
            if ctx.has(ContextKey::Seeds) {
                InvariantResult::Ok
            } else {
                InvariantResult::Violated(Violation::new("no seeds present"))
            }
        }
    }

    /// Invariant that forbids empty fact content.
    struct NoEmptyContent;

    impl Invariant for NoEmptyContent {
        fn name(&self) -> &'static str {
            "no_empty_content"
        }

        fn class(&self) -> InvariantClass {
            InvariantClass::Structural
        }

        fn check(&self, ctx: &dyn crate::Context) -> InvariantResult {
            for key in &[
                ContextKey::Seeds,
                ContextKey::Hypotheses,
                ContextKey::Strategies,
                ContextKey::Competitors,
                ContextKey::Evaluations,
            ] {
                for fact in ctx.get(*key) {
                    if fact.text().is_some_and(|text| text.trim().is_empty()) {
                        return InvariantResult::Violated(Violation::with_facts(
                            "empty content not allowed",
                            vec![fact.id().clone()],
                        ));
                    }
                }
            }
            InvariantResult::Ok
        }
    }

    #[test]
    fn registry_registers_invariants() {
        let mut registry = InvariantRegistry::new();
        let id1 = registry.register(RequireSeeds);
        let id2 = registry.register(NoEmptyContent);

        assert_eq!(registry.count(), 2);
        assert_ne!(id1, id2);
    }

    #[test]
    fn acceptance_invariant_passes_with_seeds() {
        let mut registry = InvariantRegistry::new();
        registry.register(RequireSeeds);

        let mut ctx = ContextState::new();
        let _ = ctx.add_fact(crate::context::new_fact(ContextKey::Seeds, "s1", "value"));

        assert!(registry.check_acceptance(&ctx).is_ok());
    }

    #[test]
    fn acceptance_invariant_fails_without_seeds() {
        let mut registry = InvariantRegistry::new();
        registry.register(RequireSeeds);

        let ctx = ContextState::new();
        let result = registry.check_acceptance(&ctx);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.invariant_name, "require_seeds");
        assert_eq!(err.class, InvariantClass::Acceptance);
    }

    #[test]
    fn structural_invariant_catches_empty_content() {
        let mut registry = InvariantRegistry::new();
        registry.register(NoEmptyContent);

        let mut ctx = ContextState::new();
        let _ = ctx.add_fact(crate::context::new_fact(ContextKey::Seeds, "bad", "   ")); // Empty after trim

        let result = registry.check_structural(&ctx);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .violation
                .fact_ids
                .contains(&"bad".into())
        );
    }

    #[test]
    fn different_classes_checked_independently() {
        let mut registry = InvariantRegistry::new();
        registry.register(RequireSeeds); // Acceptance
        registry.register(NoEmptyContent); // Structural

        let ctx = ContextState::new();

        // Structural passes (no facts to check)
        assert!(registry.check_structural(&ctx).is_ok());

        // Acceptance fails (no seeds)
        assert!(registry.check_acceptance(&ctx).is_err());
    }
}
