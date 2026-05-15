// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Engine benchmarks.
//!
//! Measures core convergence loop performance:
//! - Single-cycle latency with 1 suggestor
//! - Multi-suggestor cascades at varying widths
//! - Budget pressure (near-ceiling convergence)
//! - Context scale (1000+ pre-seeded facts)
//!
//! Run with: `cargo bench -p converge-core`

use converge_core::{
    AgentEffect, ContextKey, ContextState, Engine, ProposedFact, Suggestor, TextPayload,
    suggestors::SeedSuggestor,
};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

// ─── Single-cycle: 1 seed suggestor, immediate convergence ───

fn bench_engine_single_cycle(c: &mut Criterion) {
    c.bench_function("engine_single_cycle", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().expect("create runtime");
            rt.block_on(async {
                let mut engine = Engine::new();
                engine.register_suggestor(SeedSuggestor::new("seed-1", "test data"));

                engine
                    .run(ContextState::new())
                    .await
                    .expect("should converge")
            })
        });
    });
}

// ─── Multi-suggestor cascades at varying widths (1, 5, 20) ───

fn bench_engine_multi_suggestor(c: &mut Criterion) {
    struct SimpleProposer {
        name: String,
        key: ContextKey,
    }

    #[async_trait::async_trait]
    impl Suggestor for SimpleProposer {
        fn name(&self) -> &str {
            &self.name
        }

        fn dependencies(&self) -> &[ContextKey] {
            &[]
        }

        fn accepts(&self, _ctx: &dyn converge_core::Context) -> bool {
            true
        }

        async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
            AgentEffect::with_proposal(ProposedFact::new(
                self.key,
                format!("{}-proposal", self.name),
                TextPayload::new(format!("proposal from {}", self.name)),
                self.name.clone(),
            ))
        }
    }

    let mut group = c.benchmark_group("engine_multi_suggestor");

    for n in [1, 5, 20] {
        group.bench_with_input(BenchmarkId::new("suggestors", n), &n, |b, &n| {
            b.iter(|| {
                let rt = tokio::runtime::Runtime::new().expect("create runtime");
                rt.block_on(async {
                    let mut engine = Engine::new();
                    engine.register_suggestor(SeedSuggestor::new("seed", "initial"));

                    for i in 0..n {
                        engine.register_suggestor(SimpleProposer {
                            name: format!("proposer-{}", i),
                            key: ContextKey::Hypotheses,
                        });
                    }

                    engine
                        .run(ContextState::new())
                        .await
                        .expect("should converge")
                })
            });
        });
    }

    group.finish();
}

// ─── Budget pressure: run near ceiling, verify early-exit timing ───

fn bench_engine_budget_pressure(c: &mut Criterion) {
    struct InfiniteProposer {
        key: ContextKey,
    }

    #[async_trait::async_trait]
    impl Suggestor for InfiniteProposer {
        fn name(&self) -> &str {
            "InfiniteProposer"
        }

        fn dependencies(&self) -> &[ContextKey] {
            &[]
        }

        fn accepts(&self, _ctx: &dyn converge_core::Context) -> bool {
            true
        }

        async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
            AgentEffect::with_proposal(ProposedFact::new(
                self.key,
                "unlimited-1",
                TextPayload::new("will exhaust budget"),
                self.name().to_string(),
            ))
        }
    }

    c.bench_function("engine_budget_pressure_near_ceiling", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().expect("create runtime");
            rt.block_on(async {
                let mut engine = Engine::with_budget(converge_core::Budget {
                    max_cycles: 99,
                    max_facts: 5000,
                });
                engine.register_suggestor(InfiniteProposer {
                    key: ContextKey::Hypotheses,
                });

                let result = engine.run(ContextState::new()).await;
                // Should either converge or hit budget boundary
                result.ok()
            })
        });
    });
}

// ─── Context scale: engine performance with pre-seeded facts ───

fn bench_engine_large_context(c: &mut Criterion) {
    c.bench_function("engine_large_context_1000_facts", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().expect("create runtime");
            rt.block_on(async {
                let mut ctx = ContextState::new();

                // Pre-seed 1000 facts
                for i in 0..1000 {
                    let _ = ctx.add_input(
                        ContextKey::Seeds,
                        format!("seed-{}", i),
                        format!("seed content {}", i),
                    );
                }

                let mut engine = Engine::new();
                engine.register_suggestor(SeedSuggestor::new("late-seed", "added in run"));

                engine.run(ctx).await.expect("should converge")
            })
        });
    });
}

criterion_group!(
    benches,
    bench_engine_single_cycle,
    bench_engine_multi_suggestor,
    bench_engine_budget_pressure,
    bench_engine_large_context
);
criterion_main!(benches);
