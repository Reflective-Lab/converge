//! Benchmarks for assignment algorithms

use converge_optimization::assignment::{AssignmentProblem, auction, hungarian};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::hint::black_box;

fn random_costs(n: usize, seed: u64) -> Vec<Vec<i64>> {
    let mut rng = StdRng::seed_from_u64(seed);
    (0..n)
        .map(|_| (0..n).map(|_| rng.gen_range(1..1000)).collect())
        .collect()
}

fn bench_hungarian(c: &mut Criterion) {
    let mut group = c.benchmark_group("hungarian");

    for size in [10, 50, 100, 200].iter() {
        let costs = random_costs(*size, 42);
        let problem = AssignmentProblem::from_costs(costs);

        group.bench_with_input(BenchmarkId::from_parameter(size), &problem, |b, p| {
            b.iter(|| hungarian::solve(black_box(p)))
        });
    }

    group.finish();
}

fn bench_auction(c: &mut Criterion) {
    let mut group = c.benchmark_group("auction");

    for size in [10, 50, 100, 200].iter() {
        let costs = random_costs(*size, 42);
        let problem = AssignmentProblem::from_costs(costs);

        group.bench_with_input(BenchmarkId::from_parameter(size), &problem, |b, p| {
            b.iter(|| auction::solve(black_box(p)))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_hungarian, bench_auction);
criterion_main!(benches);
