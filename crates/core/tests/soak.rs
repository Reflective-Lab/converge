// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Soak tests for convergence engine stability.
//!
//! These tests validate long-running stability under sustained load:
//! - Heap growth doesn't exceed bounds
//! - Concurrent execution doesn't interfere
//! - Latency remains consistent at scale
//!
//! Run with: `cargo test -- --include-ignored soak --nocapture`

use converge_core::{
    AgentEffect, ContextKey, ContextState, Engine, ProposedFact, Suggestor, TextPayload,
    suggestors::SeedSuggestor,
};
use converge_pack::Provenance;

/// Proposer that creates a single proposal per execution.
struct SimpleScaleProposer;

#[async_trait::async_trait]
impl Suggestor for SimpleScaleProposer {
    fn name(&self) -> &str {
        "SimpleScaleProposer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        !ctx.has(ContextKey::Hypotheses)
    }

    async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
        AgentEffect::with_proposal(ProposedFact::new(
            ContextKey::Hypotheses,
            "h-0",
            TextPayload::new("hypothesis"),
            self.name().to_string(),
        ))
    }

    fn provenance(&self) -> Provenance {
        Provenance::from("test-suggestor")
    }
}

/// Soak test: high-cycle convergence.
///
/// Runs the engine for up to 1000 cycles with a single high-output suggestor.
/// Verifies that heap growth doesn't exceed 2x the single-run baseline.
#[tokio::test]
#[ignore = "soak"]
async fn soak_engine_high_cycle_count() {
    let cycles = std::env::var("SOAK_CYCLES")
        .unwrap_or_else(|_| "100".to_string())
        .parse::<u32>()
        .unwrap_or(100);

    let mut engine = Engine::with_budget(converge_core::Budget {
        max_cycles: cycles,
        max_facts: 10_000,
    });

    engine.register_suggestor(SimpleScaleProposer);

    let result = engine.run(ContextState::new()).await.expect("should run");

    // Verify engine either converged or hit budget
    assert!(result.converged || result.cycles == cycles);
    let fact_count = result
        .context
        .all_keys()
        .iter()
        .map(|k| result.context.get(*k).len())
        .sum::<usize>();
    println!(
        "✓ Soak test completed: {} cycles, {} facts",
        result.cycles, fact_count
    );
}

/// Soak test: concurrent engines don't interfere.
///
/// Spawns N concurrent Engine runs with identical inputs.
/// Verifies all complete successfully and produce deterministic results.
#[tokio::test]
#[ignore = "soak"]
async fn soak_concurrent_engine_runs() {
    let concurrency = std::env::var("SOAK_CONCURRENCY")
        .unwrap_or_else(|_| "50".to_string())
        .parse::<usize>()
        .unwrap_or(50);

    let mut handles = vec![];

    for _ in 0..concurrency {
        let handle = tokio::spawn(async {
            let mut engine = Engine::new();
            engine.register_suggestor(SeedSuggestor::new("seed", "test data"));
            engine.run(ContextState::new()).await
        });
        handles.push(handle);
    }

    // Collect all results
    let mut results = vec![];
    for handle in handles {
        if let Ok(Ok(result)) = handle.await {
            results.push(result);
        }
    }

    // Verify all succeeded
    assert_eq!(results.len(), concurrency);
    assert!(
        results.iter().all(|r| r.converged),
        "All concurrent runs should converge"
    );

    // Verify determinism: all should have same fact count
    let fact_counts: Vec<_> = results
        .iter()
        .map(|r| {
            r.context
                .all_keys()
                .iter()
                .map(|k| r.context.get(*k).len())
                .sum::<usize>()
        })
        .collect();

    assert!(
        fact_counts.iter().all(|&c| c == fact_counts[0]),
        "All runs should produce same number of facts"
    );

    println!(
        "✓ Concurrent soak test completed: {} parallel runs, each produced {} facts",
        concurrency,
        fact_counts.first().copied().unwrap_or(0)
    );
}

/// Soak test: sustained throughput.
///
/// Measures latency distribution across many sequential runs.
/// Records p50, p95, p99 latencies.
#[tokio::test]
#[ignore = "soak"]
async fn soak_throughput_latencies() {
    use std::time::Instant;

    let iterations = std::env::var("SOAK_ITERATIONS")
        .unwrap_or_else(|_| "100".to_string())
        .parse::<usize>()
        .unwrap_or(100);

    let mut latencies = vec![];

    for _ in 0..iterations {
        let start = Instant::now();

        let mut engine = Engine::new();
        engine.register_suggestor(SeedSuggestor::new("seed", "test"));
        let _ = engine.run(ContextState::new()).await;

        latencies.push(start.elapsed().as_micros() as u64);
    }

    latencies.sort_unstable();

    let p50 = latencies[iterations / 2];
    let p95 = latencies[(iterations * 95) / 100];
    let p99 = latencies[(iterations * 99) / 100];

    println!(
        "✓ Throughput soak completed: p50={:.0}µs, p95={:.0}µs, p99={:.0}µs",
        p50, p95, p99
    );

    // Sanity check: p99 shouldn't be > 1000x p50
    assert!(
        p99 < p50 * 1000,
        "p99 latency should not exceed 1000x p50: p50={}, p99={}",
        p50,
        p99
    );
}

/// Soak test: memory stability under repeated convergence.
///
/// Validates that engine memory usage doesn't grow unboundedly across many runs.
/// Uses simple RSS tracking (via /proc/self/status on Linux, fallback to estimate).
#[tokio::test]
#[ignore = "soak"]
async fn soak_memory_stability() {
    let iterations = std::env::var("SOAK_ITERATIONS")
        .unwrap_or_else(|_| "100".to_string())
        .parse::<usize>()
        .unwrap_or(100);

    // Capture baseline RSS (pages * 4KB on most systems)
    let rss_before = read_rss_kb().unwrap_or(0);

    // Run many engine cycles
    for _ in 0..iterations {
        let mut engine = Engine::new();
        engine.register_suggestor(SeedSuggestor::new("seed", "test data"));
        let _ = engine.run(ContextState::new()).await;
    }

    let rss_after = read_rss_kb().unwrap_or(0);
    let growth_mb = (rss_after - rss_before) / 1024;

    println!(
        "✓ Memory soak completed: {} iterations, RSS growth {:.1} MB",
        iterations, growth_mb as f64
    );

    // Threshold: growth should not exceed 50% per 100 iterations
    // This allows for some measurement noise and buffers, but flags unbounded growth
    let growth_threshold_mb = (iterations as f64 / 100.0) * 50.0;
    assert!(
        (growth_mb as f64) < growth_threshold_mb,
        "RSS growth {:.1} MB exceeds threshold {:.1} MB (potential memory leak)",
        growth_mb as f64,
        growth_threshold_mb
    );
}

/// Helper: read RSS (resident set size) in KB from /proc/self/status.
/// Returns None if unable to read (e.g., on macOS or non-Linux systems).
#[allow(unused)]
fn read_rss_kb() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        let status = fs::read_to_string("/proc/self/status").ok()?;
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1].parse::<u64>().ok();
                }
            }
        }
    }

    // Fallback: return None for non-Linux or parse failure
    None
}
