// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! LLM-enabled Meeting Scheduler use case.

use crate::llm_utils::{create_mock_llm_agent, requirements};
use crate::mock::{MockChatBackend, MockResponse};
use converge_core::{ContextKey, Engine};
use std::sync::Arc;

/// Sets up LLM-enabled Meeting Scheduler agents with mock backends (for testing).
#[must_use]
pub fn setup_mock_llm_meeting_scheduler(engine: &mut Engine) -> Vec<Arc<MockChatBackend>> {
    let mut providers = Vec::new();

    // Availability Retrieval Suggestor: Fast extraction
    let (agent, backend) = create_mock_llm_agent(
        "AvailabilityRetrievalAgent",
        "You extract participant availability from calendars.",
        "Extract availability: {context}",
        ContextKey::Signals,
        vec![ContextKey::Seeds],
        requirements::fast_extraction(),
        vec![MockResponse::success(
            "Alice: Available Mon 10-12, Tue 14-16 | Bob: Available Mon 9-11, Wed 10-12",
            0.8,
        )],
    );
    engine.register_suggestor(agent);
    providers.push(backend);

    // Time Zone Normalization Suggestor: Fast processing
    let (agent, backend) = create_mock_llm_agent(
        "TimeZoneNormalizationAgent",
        "You normalize time zones to UTC.",
        "Normalize time zones: {context}",
        ContextKey::Signals,
        vec![ContextKey::Signals],
        requirements::fast_extraction(),
        vec![MockResponse::success(
            "Normalized: All times in UTC | Alice: UTC+1, Bob: UTC-5",
            0.85,
        )],
    );
    engine.register_suggestor(agent);
    providers.push(backend);

    // Slot Optimization Suggestor: Analysis for optimization
    let (agent, backend) = create_mock_llm_agent(
        "SlotOptimizationAgent",
        "You optimize meeting time slots based on availability.",
        "Optimize slots: {context}",
        ContextKey::Strategies,
        vec![ContextKey::Signals, ContextKey::Constraints],
        requirements::analysis(),
        vec![MockResponse::success(
            "Optimal slot: Mon 10:00 UTC | Participants: Alice, Bob | Duration: 60 min",
            0.9,
        )],
    );
    engine.register_suggestor(agent);
    providers.push(backend);

    providers
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::ContextState;
    use converge_core::suggestors::SeedSuggestor;

    #[tokio::test]
    async fn mock_llm_meeting_scheduler_converges() {
        let mut engine = Engine::new();
        engine.register_suggestor(SeedSuggestor::new("participants", "Alice, Bob"));

        let _providers = setup_mock_llm_meeting_scheduler(&mut engine);

        let result = engine
            .run(ContextState::new())
            .await
            .expect("should converge");

        assert!(result.converged);
        // LLM agents emit proposals that get promoted to facts on their target keys
        let signals = result.context.get(ContextKey::Signals);
        let strategies = result.context.get(ContextKey::Strategies);
        assert!(
            !signals.is_empty() || !strategies.is_empty(),
            "At least one LLM agent should have produced facts"
        );
    }
}
