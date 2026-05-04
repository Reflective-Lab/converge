//! Benchmark for native Varisat-based CP
//!
//! Run with: cargo bench --features sat -- cp_comparison

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

#[cfg(feature = "sat")]
mod native {
    use converge_optimization::cp::{CpModel, CpStatus};

    pub fn solve_nqueens(n: i64) -> bool {
        let mut model = CpModel::new();

        let queens: Vec<_> = (0..n)
            .map(|i| model.new_int_var(0, n - 1, &format!("q{}", i)))
            .collect();

        model.add_all_different(&queens);

        for i in 0..(n as usize) {
            for j in (i + 1)..(n as usize) {
                let diff = (j - i) as i64;
                let _aux_plus = model.new_int_var(0, n - 1 + diff, &format!("d+{}_{}", i, j));
                let _aux_minus = model.new_int_var(0, n - 1 + diff, &format!("d-{}_{}", i, j));
            }
        }

        let solution = model.solve();
        solution.status.is_success()
    }

    pub fn solve_linear_eq(n: usize) -> (CpStatus, i64) {
        let mut model = CpModel::new();

        let vars: Vec<_> = (0..n)
            .map(|i| model.new_int_var(0, 100, &format!("x{}", i)))
            .collect();

        let coeffs: Vec<i64> = vec![1; n];
        let target = (n * 50) as i64;
        model.add_linear_eq(&vars, &coeffs, target);

        model.minimize(&vars[0..1], &[1]);

        let solution = model.solve();
        (solution.status, solution.objective_value.unwrap_or(0))
    }

    pub fn solve_simple_satisfaction(n: usize) -> CpStatus {
        let mut model = CpModel::new();

        let vars: Vec<_> = (0..n)
            .map(|i| model.new_int_var(1, n as i64, &format!("x{}", i)))
            .collect();

        model.add_all_different(&vars);

        let solution = model.solve();
        solution.status
    }
}

#[cfg(feature = "sat")]
fn bench_cp_native(c: &mut Criterion) {
    let mut group = c.benchmark_group("CP Native");

    for n in [3, 4, 5, 6] {
        group.bench_with_input(BenchmarkId::new("native/all_different", n), &n, |b, &n| {
            b.iter(|| native::solve_simple_satisfaction(n));
        });
    }

    for n in [2, 3, 4, 5] {
        group.bench_with_input(BenchmarkId::new("native/linear_eq", n), &n, |b, &n| {
            b.iter(|| native::solve_linear_eq(n));
        });
    }

    for n in [4, 5, 6] {
        group.bench_with_input(BenchmarkId::new("native/nqueens", n), &n, |b, &n| {
            b.iter(|| native::solve_nqueens(n));
        });
    }

    group.finish();
}

#[cfg(feature = "sat")]
criterion_group!(benches, bench_cp_native);

#[cfg(feature = "sat")]
criterion_main!(benches);

#[cfg(not(feature = "sat"))]
fn main() {
    eprintln!("This benchmark requires the 'sat' feature");
    eprintln!("Run with: cargo bench --features sat -- cp_comparison");
}
