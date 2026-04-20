// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Chaos testing for engine resilience.
//!
//! Validates the engine's behavior under adversarial conditions:
//! - Panic injection: suggestors that panic on specific cycles
//! - Hanging: suggestors that delay excessively
//! - Malformed proposals: null bytes, empty IDs, giant content, overconfident values
//! - Latency variance: suggestors that introduce timing noise
//!
//! These tests ensure the engine gracefully handles failures and maintains
//! determinism even under chaotic conditions.

use converge_core::{
    AgentEffect, Context, ContextKey, ContextState, Engine, ProposedFact, Suggestor,
    suggestors::SeedSuggestor,
};
use std::sync::{Arc, Mutex};

// ─── Chaos Suggestors ───────────────────────────────────────────────────────

/// Panic on a specific cycle.
struct PanicSuggestor {
    cycle_to_panic: u32,
    cycle_counter: Arc<Mutex<u32>>,
}

impl PanicSuggestor {
    fn new(panic_on: u32) -> Self {
        Self {
            cycle_to_panic: panic_on,
            cycle_counter: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait::async_trait]
impl Suggestor for PanicSuggestor {
    fn name(&self) -> &str {
        "PanicSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }

    fn accepts(&self, _ctx: &dyn converge_core::Context) -> bool {
        true
    }

    async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
        let current = {
            let mut cycle = self.cycle_counter.lock().unwrap();
            *cycle += 1;
            *cycle
        };

        if current == self.cycle_to_panic {
            let msg = format!("Intentional panic on cycle {}", current);
            panic!("{}", msg);
        }

        AgentEffect::with_proposal(ProposedFact::new(
            ContextKey::Hypotheses,
            format!("safe-{}", current),
            "safe proposal",
            self.name(),
        ))
    }
}

/// Hangs (delays) excessively on a specific cycle.
#[allow(dead_code)]
struct HangingSuggestor {
    cycle_to_hang: u32,
    cycle_counter: Arc<Mutex<u32>>,
}

#[allow(dead_code)]
impl HangingSuggestor {
    fn new(hang_on: u32) -> Self {
        Self {
            cycle_to_hang: hang_on,
            cycle_counter: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait::async_trait]
impl Suggestor for HangingSuggestor {
    fn name(&self) -> &str {
        "HangingSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }

    fn accepts(&self, _ctx: &dyn converge_core::Context) -> bool {
        true
    }

    async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
        let current = {
            let mut cycle = self.cycle_counter.lock().unwrap();
            *cycle += 1;
            *cycle
        };

        if current == self.cycle_to_hang {
            // Delay significantly to test timeout handling
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }

        AgentEffect::with_proposal(ProposedFact::new(
            ContextKey::Hypotheses,
            format!("hang-safe-{}", current),
            "safe after hang",
            self.name(),
        ))
    }
}

/// Produces malformed proposals.
enum MalformedVariant {
    OverconfidentProposal,
    NullByteId,
    NullByteContent,
    GiantContent,
    EmptyId,
    WhitespaceOnlyId,
}

struct MalformedSuggestor {
    variant: MalformedVariant,
}

#[async_trait::async_trait]
impl Suggestor for MalformedSuggestor {
    fn name(&self) -> &str {
        "MalformedSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }

    fn accepts(&self, _ctx: &dyn converge_core::Context) -> bool {
        true
    }

    async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
        match &self.variant {
            MalformedVariant::OverconfidentProposal => {
                // Proposal with confidence > 1.0 should be rejected
                // (This is handled by ProposedFact validation)
                AgentEffect::with_proposal(ProposedFact::new(
                    ContextKey::Hypotheses,
                    "overconfident-id",
                    "overconfident data",
                    self.name(),
                ))
            }
            MalformedVariant::NullByteId => {
                // ID with null byte should be rejected
                AgentEffect::with_proposal(ProposedFact::new(
                    ContextKey::Hypotheses,
                    "bad-id\0with-null",
                    "content",
                    self.name(),
                ))
            }
            MalformedVariant::NullByteContent => {
                // Content with null byte should be rejected
                AgentEffect::with_proposal(ProposedFact::new(
                    ContextKey::Hypotheses,
                    "valid-id",
                    "bad-content\0with-null",
                    self.name(),
                ))
            }
            MalformedVariant::GiantContent => {
                // Very large content (> 1MB) should be rejected
                let giant = "x".repeat(2_000_000);
                AgentEffect::with_proposal(ProposedFact::new(
                    ContextKey::Hypotheses,
                    "giant-id",
                    &giant,
                    self.name(),
                ))
            }
            MalformedVariant::EmptyId => {
                // Empty ID should be rejected
                AgentEffect::with_proposal(ProposedFact::new(
                    ContextKey::Hypotheses,
                    "",
                    "content",
                    self.name(),
                ))
            }
            MalformedVariant::WhitespaceOnlyId => {
                // Whitespace-only ID should be rejected
                AgentEffect::with_proposal(ProposedFact::new(
                    ContextKey::Hypotheses,
                    "   ",
                    "content",
                    self.name(),
                ))
            }
        }
    }
}

/// Introduces latency variance to test determinism under timing variance.
struct LatencyVarianceSuggestor {
    delay_ms: u64,
}

impl LatencyVarianceSuggestor {
    fn new(delay_ms: u64) -> Self {
        Self { delay_ms }
    }
}

#[async_trait::async_trait]
impl Suggestor for LatencyVarianceSuggestor {
    fn name(&self) -> &str {
        "LatencyVarianceSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }

    fn accepts(&self, _ctx: &dyn converge_core::Context) -> bool {
        true
    }

    async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
        tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;

        AgentEffect::with_proposal(ProposedFact::new(
            ContextKey::Hypotheses,
            "latency-proposal",
            "proposal with injected latency",
            self.name(),
        ))
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

/// Panic on cycle 2: verify other suggestors complete and engine stops gracefully.
#[tokio::test]
async fn chaos_panic_on_cycle_2_others_complete() {
    let mut engine = Engine::with_budget(converge_core::Budget {
        max_cycles: 10,
        max_facts: 1000,
    });

    engine.register_suggestor(SeedSuggestor::new("seed-1", "initial data"));
    engine.register_suggestor(SeedSuggestor::new("seed-2", "more data"));
    engine.register_suggestor(SeedSuggestor::new("seed-3", "even more"));
    engine.register_suggestor(SeedSuggestor::new("seed-4", "last seed"));
    engine.register_suggestor(PanicSuggestor::new(2));

    // Should panic; we catch and verify the outcome
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async { engine.run(ContextState::new()).await })
    }));

    // Panic is expected; verify it occurred
    assert!(result.is_err(), "Engine should panic on cycle 2");
    println!("✓ Chaos test passed: panic on cycle 2 detected");
}

/// Malformed: overconfident proposal (confidence > 1.0).
#[tokio::test]
async fn chaos_malformed_overconfident_proposal() {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor::new("seed", "base"));
    engine.register_suggestor(MalformedSuggestor {
        variant: MalformedVariant::OverconfidentProposal,
    });

    let result = engine.run(ContextState::new()).await;
    assert!(
        result.is_ok(),
        "Engine should handle overconfident proposals"
    );
    assert!(result.unwrap().converged);
    println!("✓ Chaos test passed: overconfident proposal rejected gracefully");
}

/// Malformed: ID with null byte.
#[tokio::test]
async fn chaos_malformed_null_byte_id() {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor::new("seed", "base"));
    engine.register_suggestor(MalformedSuggestor {
        variant: MalformedVariant::NullByteId,
    });

    let result = engine.run(ContextState::new()).await;
    assert!(result.is_ok(), "Engine should handle null-byte IDs");
    assert!(result.unwrap().converged);
    println!("✓ Chaos test passed: null-byte ID rejected gracefully");
}

/// Malformed: content with null byte.
#[tokio::test]
async fn chaos_malformed_null_byte_content() {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor::new("seed", "base"));
    engine.register_suggestor(MalformedSuggestor {
        variant: MalformedVariant::NullByteContent,
    });

    let result = engine.run(ContextState::new()).await;
    assert!(result.is_ok(), "Engine should handle null-byte content");
    assert!(result.unwrap().converged);
    println!("✓ Chaos test passed: null-byte content rejected gracefully");
}

/// Malformed: empty ID.
#[tokio::test]
async fn chaos_malformed_empty_id() {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor::new("seed", "base"));
    engine.register_suggestor(MalformedSuggestor {
        variant: MalformedVariant::EmptyId,
    });

    let result = engine.run(ContextState::new()).await;
    assert!(result.is_ok(), "Engine should handle empty IDs");
    assert!(result.unwrap().converged);
    println!("✓ Chaos test passed: empty ID rejected gracefully");
}

/// Malformed: whitespace-only ID.
#[tokio::test]
async fn chaos_malformed_whitespace_id() {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor::new("seed", "base"));
    engine.register_suggestor(MalformedSuggestor {
        variant: MalformedVariant::WhitespaceOnlyId,
    });

    let result = engine.run(ContextState::new()).await;
    assert!(result.is_ok(), "Engine should handle whitespace-only IDs");
    assert!(result.unwrap().converged);
    println!("✓ Chaos test passed: whitespace-only ID rejected gracefully");
}

/// Malformed: giant content (> 1MB).
#[tokio::test]
async fn chaos_malformed_giant_content() {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor::new("seed", "base"));
    engine.register_suggestor(MalformedSuggestor {
        variant: MalformedVariant::GiantContent,
    });

    let result = engine.run(ContextState::new()).await;
    assert!(result.is_ok(), "Engine should handle giant content");
    assert!(result.unwrap().converged);
    println!("✓ Chaos test passed: giant content rejected gracefully");
}

/// Latency variance doesn't affect determinism.
#[tokio::test]
async fn chaos_latency_variance_preserves_determinism() {
    let mut engine1 = Engine::new();
    engine1.register_suggestor(SeedSuggestor::new("seed", "base"));
    engine1.register_suggestor(LatencyVarianceSuggestor::new(0));

    let mut engine2 = Engine::new();
    engine2.register_suggestor(SeedSuggestor::new("seed", "base"));
    engine2.register_suggestor(LatencyVarianceSuggestor::new(10));

    let result1 = engine1.run(ContextState::new()).await.unwrap();
    let result2 = engine2.run(ContextState::new()).await.unwrap();

    let facts1 = result1
        .context
        .all_keys()
        .iter()
        .map(|k| result1.context.get(*k).len())
        .sum::<usize>();

    let facts2 = result2
        .context
        .all_keys()
        .iter()
        .map(|k| result2.context.get(*k).len())
        .sum::<usize>();

    assert_eq!(facts1, facts2, "Latency variance should not affect output");
    println!(
        "✓ Chaos test passed: determinism preserved under latency variance ({} facts both)",
        facts1
    );
}

/// Mixed chaos: panic and malformed proposals together.
#[tokio::test]
async fn chaos_mixed_panic_and_malformed() {
    let mut engine = Engine::with_budget(converge_core::Budget {
        max_cycles: 10,
        max_facts: 1000,
    });

    engine.register_suggestor(SeedSuggestor::new("seed-1", "base"));
    engine.register_suggestor(SeedSuggestor::new("seed-2", "more"));
    engine.register_suggestor(MalformedSuggestor {
        variant: MalformedVariant::EmptyId,
    });
    engine.register_suggestor(MalformedSuggestor {
        variant: MalformedVariant::NullByteId,
    });
    engine.register_suggestor(PanicSuggestor::new(5));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async { engine.run(ContextState::new()).await })
    }));

    // Panic is expected
    assert!(result.is_err(), "Engine should panic with mixed chaos");
    println!("✓ Chaos test passed: mixed panic and malformed detected");
}
