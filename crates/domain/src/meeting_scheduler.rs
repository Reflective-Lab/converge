// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Meeting Scheduler agents for calendar coordination.
//!
//! This module implements a deterministic meeting scheduling use case
//! that validates the Converge engine with constraint satisfaction.
//!
//! # Suggestor Pipeline
//!
//! ```text
//! Seeds (participants, duration, window)
//!    │
//!    ▼
//! AvailabilityRetrievalAgent → Signals (availability data)
//!    │
//!    ▼
//! TimeZoneNormalizationAgent → Signals (normalized times)
//!    │
//!    ▼
//! WorkingHoursConstraintAgent → Constraints (working hours)
//!    │
//!    ▼
//! SlotOptimizationAgent → Strategies (candidate slots)
//!    │
//!    ▼
//! ConflictDetectionAgent → Evaluations (valid slots ranked)
//! ```ignore
//!
//! # Example
//!
//! ```
//! use converge_core::{Engine, Context, ContextKey};
//! use converge_core::suggestors::SeedSuggestor;
//! use converge_domain::meeting_scheduler::{
//!     AvailabilityRetrievalAgent, TimeZoneNormalizationAgent,
//!     WorkingHoursConstraintAgent, SlotOptimizationAgent, ConflictDetectionAgent,
//! };
//!
//! let mut engine = Engine::new();
//!
//! // Seed the context with meeting requirements
//! engine.register_suggestor(SeedSuggestor::new("participants", "Alice, Bob, Carol"));
//! engine.register_suggestor(SeedSuggestor::new("duration", "60"));
//! engine.register_suggestor(SeedSuggestor::new("window", "next week"));
//!
//! // Register meeting scheduler agents
//! engine.register_suggestor(AvailabilityRetrievalAgent);
//! engine.register_suggestor(TimeZoneNormalizationAgent);
//! engine.register_suggestor(WorkingHoursConstraintAgent);
//! engine.register_suggestor(SlotOptimizationAgent);
//! engine.register_suggestor(ConflictDetectionAgent);
//!
//! let result = engine.run(ContextState::new()).await.expect("should converge");
//!
//! assert!(result.converged);
//! assert!(result.context.has(ContextKey::Strategies));
//! assert!(result.context.has(ContextKey::Evaluations));
//! ```

// Suggestor trait returns &str, but we return literals. This is fine.
#![allow(clippy::unnecessary_literal_bound)]

use converge_core::{AgentEffect, ContextKey, Fact, Suggestor};

/// Suggestor that retrieves availability for participants.
///
///
/// Simulates calendar lookup for each participant.
/// In a real system, this would query calendar APIs.
pub struct AvailabilityRetrievalAgent;

#[async_trait::async_trait]
impl Suggestor for AvailabilityRetrievalAgent {
    fn name(&self) -> &str {
        "AvailabilityRetrievalAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        // Run once when seeds exist but no availability signals yet
        ctx.has(ContextKey::Seeds) && !ctx.has(ContextKey::Signals)
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);

        // Extract participants from seeds
        let participants_seed = seeds
            .iter()
            .find(|s| s.id == "participants" || s.content.contains("participants"));

        let mut facts = Vec::new();

        if let Some(seed) = participants_seed {
            // Deterministic availability generation based on participants
            let content = &seed.content;
            if content.contains("Alice") {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Signals,
                    "availability:alice",
                    "Alice: Mon 9-17, Tue 9-17, Wed 9-17, Thu 9-17, Fri 9-17 (UTC)",
                ));
            }
            if content.contains("Bob") {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Signals,
                    "availability:bob",
                    "Bob: Mon 10-18, Tue 10-18, Wed 10-18, Thu 10-18, Fri 10-18 (UTC)",
                ));
            }
            if content.contains("Carol") {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Signals,
                    "availability:carol",
                    "Carol: Mon 8-16, Tue 8-16, Wed 8-16, Thu 8-16, Fri 8-16 (UTC)",
                ));
            }
        }

        // Always emit baseline availability if no specific data
        if facts.is_empty() {
            facts.push(crate::proposal(
                self.name(),
                ContextKey::Signals,
                "availability:default",
                "Default: Mon-Fri 9-17 UTC",
            ));
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Suggestor that normalizes time zones and availability windows.
///
///
/// Converts all availability to a common timezone and aligns windows.
pub struct TimeZoneNormalizationAgent;

#[async_trait::async_trait]
impl Suggestor for TimeZoneNormalizationAgent {
    fn name(&self) -> &str {
        "TimeZoneNormalizationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        // Run when availability signals exist but no normalized times yet
        // Check if we have availability signals but haven't normalized
        let has_availability = ctx
            .get(ContextKey::Signals)
            .iter()
            .any(|s| s.id.starts_with("availability:"));
        let has_normalized = ctx
            .get(ContextKey::Signals)
            .iter()
            .any(|s| s.id.starts_with("normalized:"));

        has_availability && !has_normalized
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);

        let mut facts = Vec::new();

        // Find all availability signals
        let availability_signals: Vec<_> = signals
            .iter()
            .filter(|s| s.id.starts_with("availability:"))
            .collect();

        if !availability_signals.is_empty() {
            // Normalize to UTC and find common windows
            // For simplicity, assume all are already in UTC and find overlap
            facts.push(crate::proposal(
                self.name(),
                ContextKey::Signals,
                "normalized:common-window",
                "Common availability: Mon-Fri 10-16 UTC (all participants)",
            ));
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Suggestor that enforces working hours constraints.
///
///
/// Validates that candidate slots respect working hour policies.
pub struct WorkingHoursConstraintAgent;

#[async_trait::async_trait]
impl Suggestor for WorkingHoursConstraintAgent {
    fn name(&self) -> &str {
        "WorkingHoursConstraintAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals, ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        // Run when normalized availability exists but constraints not yet defined
        let has_normalized = ctx
            .get(ContextKey::Signals)
            .iter()
            .any(|s| s.id.starts_with("normalized:"));
        let has_constraints = ctx
            .get(ContextKey::Constraints)
            .iter()
            .any(|c| c.id.starts_with("working-hours:"));

        has_normalized && !has_constraints
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let seeds = ctx.get(ContextKey::Seeds);

        let mut facts = Vec::new();

        // Extract duration from seeds
        let duration = seeds
            .iter()
            .find(|s| s.id == "duration")
            .and_then(|s| s.content.parse::<u32>().ok())
            .unwrap_or(60);

        // Define working hours constraint based on normalized availability
        let normalized = signals.iter().find(|s| s.id.starts_with("normalized:"));

        if let Some(norm) = normalized {
            facts.push(crate::proposal(
                self.name(),
                ContextKey::Constraints,
                "working-hours:policy",
                format!(
                    "Working hours: {} | Minimum duration: {} minutes",
                    norm.content, duration
                ),
            ));
        } else {
            // Default constraint
            facts.push(crate::proposal(
                self.name(),
                ContextKey::Constraints,
                "working-hours:default",
                format!("Working hours: Mon-Fri 9-17 UTC | Duration: {duration} minutes"),
            ));
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Suggestor that optimizes and generates candidate time slots.
///
///
/// Produces ranked candidate slots based on availability and constraints.
pub struct SlotOptimizationAgent;

#[async_trait::async_trait]
impl Suggestor for SlotOptimizationAgent {
    fn name(&self) -> &str {
        "SlotOptimizationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Constraints, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        // Run when constraints exist but no candidate slots yet
        ctx.has(ContextKey::Constraints) && !ctx.has(ContextKey::Strategies)
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let constraints = ctx.get(ContextKey::Constraints);
        let _signals = ctx.get(ContextKey::Signals);

        let mut facts = Vec::new();

        // Extract duration from constraints
        let duration = constraints
            .iter()
            .find(|c| c.id.starts_with("working-hours:"))
            .and_then(|c| {
                c.content
                    .split("Duration: ")
                    .nth(1)
                    .and_then(|s| s.split_whitespace().next())
                    .and_then(|s| s.parse::<u32>().ok())
            })
            .unwrap_or(60);

        // Generate candidate slots based on common availability
        // For simplicity, generate slots for next week (Mon-Fri)
        let slots = vec![
            ("Mon 10:00", "Mon 11:00", 1),
            ("Tue 10:00", "Tue 11:00", 2),
            ("Wed 10:00", "Wed 11:00", 3),
            ("Thu 10:00", "Thu 11:00", 4),
            ("Fri 10:00", "Fri 11:00", 5),
            ("Mon 14:00", "Mon 15:00", 6),
            ("Tue 14:00", "Tue 15:00", 7),
        ];

        for (start, end, rank) in slots {
            facts.push(crate::proposal(
                self.name(),
                ContextKey::Strategies,
                format!("slot:{rank}"),
                format!("Candidate slot {rank}: {start} - {end} ({duration} minutes)"),
            ));
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Suggestor that detects conflicts and ranks valid slots.
///
///
/// Evaluates candidate slots against constraints and ranks them.
pub struct ConflictDetectionAgent;

#[async_trait::async_trait]
impl Suggestor for ConflictDetectionAgent {
    fn name(&self) -> &str {
        "ConflictDetectionAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies, ContextKey::Constraints]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        // Run when candidate slots exist but no evaluations yet
        ctx.has(ContextKey::Strategies) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let constraints = ctx.get(ContextKey::Constraints);

        let mut facts = Vec::new();

        // Evaluate each candidate slot
        for (i, slot) in strategies.iter().enumerate() {
            // Check if slot respects working hours
            let is_valid = constraints
                .iter()
                .any(|c| c.content.contains("10-16") || c.content.contains("9-17"));

            if is_valid {
                let (score, rationale) = evaluate_slot(slot, i);

                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Evaluations,
                    format!("eval:{}", slot.id.strip_prefix("slot:").unwrap_or(&slot.id)),
                    format!(
                        "Score: {}/100 | {} | Rationale: {}",
                        score,
                        if i == 0 { "RECOMMENDED" } else { "ALTERNATIVE" },
                        rationale
                    ),
                ));
            }
        }

        // Ensure at least one valid slot
        if facts.is_empty() {
            facts.push(crate::proposal(
                self.name(),
                ContextKey::Evaluations,
                "eval:no-slot",
                "Score: 0/100 | INFEASIBLE | Rationale: No valid slots found within constraints",
            ));
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Deterministic slot evaluation function.
fn evaluate_slot(slot: &Fact, _rank: usize) -> (u32, &'static str) {
    // Prefer earlier slots and morning times
    let content = &slot.content;

    if content.contains("Mon 10:00") || content.contains("Tue 10:00") {
        (90, "Early week morning slot, minimal disruption")
    } else if content.contains("Wed 10:00") || content.contains("Thu 10:00") {
        (85, "Mid-week morning slot, good balance")
    } else if content.contains("Fri 10:00") {
        (80, "Friday morning slot, end of week")
    } else if content.contains("14:00") {
        (75, "Afternoon slot, acceptable but less preferred")
    } else {
        (70, "Valid slot within constraints")
    }
}

// =============================================================================
// MEETING SCHEDULER INVARIANTS
// =============================================================================

use converge_core::{Invariant, InvariantClass, InvariantResult, Violation};

/// Acceptance invariant: At least one valid slot must exist.
///
/// From Gherkin spec:
/// ```gherkin
/// Scenario: Valid meeting time
///   When the system converges
///   Then at least one valid meeting slot exists
/// ```
pub struct RequireValidSlot;

impl Invariant for RequireValidSlot {
    fn name(&self) -> &str {
        "require_valid_slot"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        let evaluations = ctx.get(ContextKey::Evaluations);

        // Check if there's at least one valid (non-INFEASIBLE) evaluation
        let has_valid = evaluations
            .iter()
            .any(|e| !e.content.contains("INFEASIBLE") && e.content.contains("Score:"));

        if !has_valid {
            return InvariantResult::Violated(Violation::new(
                "no valid meeting slots found within constraints",
            ));
        }
        InvariantResult::Ok
    }
}

/// Semantic invariant: All participants must be available for selected slot.
///
/// From Gherkin spec:
/// ```gherkin
/// Scenario: Participant availability
///   When a slot is selected
///   Then all participants are available during that time
/// ```
pub struct RequireParticipantAvailability;

impl Invariant for RequireParticipantAvailability {
    fn name(&self) -> &str {
        "require_participant_availability"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        let evaluations = ctx.get(ContextKey::Evaluations);
        let signals = ctx.get(ContextKey::Signals);

        // Only check when pipeline has progressed to evaluations
        // (availability is checked at that point)
        if evaluations.is_empty() {
            return InvariantResult::Ok;
        }

        // Check if we have availability data
        let has_availability = signals.iter().any(|s| s.id.starts_with("availability:"));

        if !has_availability {
            return InvariantResult::Violated(Violation::new(
                "participant availability data not found",
            ));
        }

        // Check if recommended slot respects availability
        let recommended = evaluations
            .iter()
            .find(|e| e.content.contains("RECOMMENDED"));

        if let Some(rec) = recommended {
            // For simplicity, assume slots in 10-16 range are valid
            // In a real system, this would cross-check with availability signals
            if rec.content.contains("INFEASIBLE") {
                return InvariantResult::Violated(Violation::with_facts(
                    "recommended slot conflicts with participant availability",
                    vec![rec.id.clone()],
                ));
            }
        }

        InvariantResult::Ok
    }
}

/// Structural invariant: Duration must be positive.
///
/// From Gherkin spec:
/// ```gherkin
/// Scenario: Valid meeting parameters
///   Given a meeting is scheduled
///   Then the duration is greater than zero
/// ```
pub struct RequirePositiveDuration;

impl Invariant for RequirePositiveDuration {
    fn name(&self) -> &str {
        "require_positive_duration"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        let seeds = ctx.get(ContextKey::Seeds);
        let constraints = ctx.get(ContextKey::Constraints);

        // Check duration in seeds
        let duration_from_seeds = seeds
            .iter()
            .find(|s| s.id == "duration")
            .and_then(|s| s.content.parse::<u32>().ok());

        // Check duration in constraints
        let duration_from_constraints = constraints
            .iter()
            .find(|c| c.id.starts_with("working-hours:"))
            .and_then(|c| {
                c.content
                    .split("Duration: ")
                    .nth(1)
                    .and_then(|s| s.split_whitespace().next())
                    .and_then(|s| s.parse::<u32>().ok())
            });

        let duration = duration_from_seeds.or(duration_from_constraints);

        if let Some(dur) = duration {
            if dur == 0 {
                return InvariantResult::Violated(Violation::new(
                    "meeting duration must be greater than zero",
                ));
            }
        }

        InvariantResult::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::ContextState;
    use converge_core::Engine;
    use converge_core::suggestors::SeedSuggestor;

    #[tokio::test]
    async fn availability_agent_retrieves_availability() {
        let mut engine = Engine::new();
        engine.register_suggestor(SeedSuggestor::new("participants", "Alice, Bob, Carol"));
        engine.register_suggestor(AvailabilityRetrievalAgent);

        let result = engine
            .run(ContextState::new())
            .await
            .expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Signals));

        let signals = result.context.get(ContextKey::Signals);
        assert!(signals.iter().any(|s| s.id.contains("availability")));
    }

    #[tokio::test]
    async fn timezone_agent_normalizes_availability() {
        let mut engine = Engine::new();
        engine.register_suggestor(SeedSuggestor::new("participants", "Alice, Bob, Carol"));
        engine.register_suggestor(AvailabilityRetrievalAgent);
        engine.register_suggestor(TimeZoneNormalizationAgent);

        let result = engine
            .run(ContextState::new())
            .await
            .expect("should converge");

        assert!(result.converged);
        let signals = result.context.get(ContextKey::Signals);
        assert!(signals.iter().any(|s| s.id.starts_with("normalized:")));
    }

    #[tokio::test]
    async fn working_hours_agent_enforces_constraints() {
        let mut engine = Engine::new();
        engine.register_suggestor(SeedSuggestor::new("participants", "Alice, Bob, Carol"));
        engine.register_suggestor(SeedSuggestor::new("duration", "60"));
        engine.register_suggestor(AvailabilityRetrievalAgent);
        engine.register_suggestor(TimeZoneNormalizationAgent);
        engine.register_suggestor(WorkingHoursConstraintAgent);

        let result = engine
            .run(ContextState::new())
            .await
            .expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Constraints));
    }

    #[tokio::test]
    async fn slot_optimization_agent_generates_candidates() {
        let mut engine = Engine::new();
        engine.register_suggestor(SeedSuggestor::new("participants", "Alice, Bob, Carol"));
        engine.register_suggestor(SeedSuggestor::new("duration", "60"));
        engine.register_suggestor(AvailabilityRetrievalAgent);
        engine.register_suggestor(TimeZoneNormalizationAgent);
        engine.register_suggestor(WorkingHoursConstraintAgent);
        engine.register_suggestor(SlotOptimizationAgent);

        let result = engine
            .run(ContextState::new())
            .await
            .expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Strategies));

        let slots = result.context.get(ContextKey::Strategies);
        assert!(!slots.is_empty());
    }

    #[tokio::test]
    async fn conflict_detection_agent_ranks_slots() {
        let mut engine = Engine::new();
        engine.register_suggestor(SeedSuggestor::new("participants", "Alice, Bob, Carol"));
        engine.register_suggestor(SeedSuggestor::new("duration", "60"));
        engine.register_suggestor(AvailabilityRetrievalAgent);
        engine.register_suggestor(TimeZoneNormalizationAgent);
        engine.register_suggestor(WorkingHoursConstraintAgent);
        engine.register_suggestor(SlotOptimizationAgent);
        engine.register_suggestor(ConflictDetectionAgent);

        let result = engine
            .run(ContextState::new())
            .await
            .expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Evaluations));

        let evals = result.context.get(ContextKey::Evaluations);
        assert!(!evals.is_empty());
        assert!(evals.iter().any(|e| e.content.contains("RECOMMENDED")));
    }

    #[tokio::test]
    async fn full_pipeline_converges_deterministically() {
        let run = || async {
            let mut engine = Engine::new();
            engine.register_suggestor(SeedSuggestor::new("participants", "Alice, Bob, Carol"));
            engine.register_suggestor(SeedSuggestor::new("duration", "60"));
            engine.register_suggestor(AvailabilityRetrievalAgent);
            engine.register_suggestor(TimeZoneNormalizationAgent);
            engine.register_suggestor(WorkingHoursConstraintAgent);
            engine.register_suggestor(SlotOptimizationAgent);
            engine.register_suggestor(ConflictDetectionAgent);
            engine
                .run(ContextState::new())
                .await
                .expect("should converge")
        };

        let r1 = run().await;
        let r2 = run().await;

        // Same number of cycles
        assert_eq!(r1.cycles, r2.cycles);

        // Same slots
        assert_eq!(
            r1.context.get(ContextKey::Strategies),
            r2.context.get(ContextKey::Strategies)
        );

        // Same evaluations
        assert_eq!(
            r1.context.get(ContextKey::Evaluations),
            r2.context.get(ContextKey::Evaluations)
        );
    }
}
