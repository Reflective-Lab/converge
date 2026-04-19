// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Meeting Scheduler — domain pack with constraint agents.
//!
//! Shows: domain agents, invariants, constraint satisfaction via convergence.

use std::sync::Arc;

use converge_domain::{
    AvailabilityRetrievalAgent, ConflictDetectionAgent, RequireParticipantAvailability,
    RequirePositiveDuration, RequireValidSlot, SlotOptimizationAgent, TimeZoneNormalizationAgent,
    WorkingHoursConstraintAgent,
};
use converge_experience::{InMemoryExperienceStore, StoreObserver, summarize_events};
use converge_kernel::{Context, ContextKey, Engine, EventQuery, ExperienceEvent, ExperienceStore};

fn print_experience_summary(store: &Arc<InMemoryExperienceStore>) {
    let Ok(events) = store.query_events(&EventQuery::default()) else {
        println!("\nExperience capture unavailable.");
        return;
    };

    if events.is_empty() {
        println!("\nNo experience events captured.");
        return;
    }

    let summary = summarize_events(&events);
    let mut promoted_facts = Vec::new();

    for envelope in events {
        if let ExperienceEvent::FactPromoted {
            fact_id, reason, ..
        } = envelope.event
        {
            promoted_facts.push(format!("{fact_id} ({reason})"));
        }
    }

    println!("\nExperience capture:");
    for (label, count) in summary.by_kind {
        println!("  {label}: {count}");
    }

    if !promoted_facts.is_empty() {
        println!("  promoted facts:");
        for fact in promoted_facts.into_iter().take(5) {
            println!("    - {fact}");
        }
    }
}

#[tokio::main]
async fn main() {
    println!("=== Meeting Scheduler Example ===\n");

    let mut engine = Engine::new();
    let experience_store = Arc::new(InMemoryExperienceStore::new());

    engine.register_suggestor(AvailabilityRetrievalAgent);
    engine.register_suggestor(TimeZoneNormalizationAgent);
    engine.register_suggestor(ConflictDetectionAgent);
    engine.register_suggestor(WorkingHoursConstraintAgent);
    engine.register_suggestor(SlotOptimizationAgent);

    engine.register_invariant(RequireParticipantAvailability);
    engine.register_invariant(RequirePositiveDuration);
    engine.register_invariant(RequireValidSlot);
    engine.set_event_observer(Arc::new(StoreObserver::new(experience_store.clone())));

    let mut ctx = Context::new();
    let _ = ctx.add_input(
        ContextKey::Seeds,
        "request-1",
        serde_json::json!({
            "participants": ["alice@example.com", "bob@example.com"],
            "duration_minutes": 60,
            "preferred_window": "2026-03-25T09:00..2026-03-25T17:00",
            "timezone": "Europe/Stockholm"
        })
        .to_string(),
    );

    println!("Scheduling request seeded.\n");

    match engine.run(ctx).await {
        Ok(result) => {
            println!("Converged: {}", result.converged);
            println!("Cycles:    {}", result.cycles);
            println!("Stop:      {:?}\n", result.stop_reason);

            for fact in result.context.get(ContextKey::Hypotheses) {
                println!("Proposed slot: {}", fact.content);
            }
        }
        Err(e) => {
            println!("Scheduling failed: {e}");
            println!("(This is expected if availability data is not seeded)");
        }
    }

    print_experience_summary(&experience_store);

    println!("\n=== Done ===");
}
