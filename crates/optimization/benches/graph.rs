//! Benchmarks for graph algorithms

use converge_optimization::graph::dijkstra;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use petgraph::graph::DiGraph;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::hint::black_box;

fn random_graph(nodes: usize, edges: usize, seed: u64) -> DiGraph<(), i64> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut graph = DiGraph::new();

    let node_indices: Vec<_> = (0..nodes).map(|_| graph.add_node(())).collect();

    for _ in 0..edges {
        let from = rng.gen_range(0..nodes);
        let to = rng.gen_range(0..nodes);
        if from != to {
            let cost = rng.gen_range(1..100);
            graph.add_edge(node_indices[from], node_indices[to], cost);
        }
    }

    graph
}

fn bench_dijkstra(c: &mut Criterion) {
    let mut group = c.benchmark_group("dijkstra");

    for (nodes, edges) in [(100, 500), (500, 2500), (1000, 5000)].iter() {
        let graph = random_graph(*nodes, *edges, 42);
        let source = graph.node_indices().next().unwrap();

        group.bench_with_input(BenchmarkId::new("nodes", nodes), &graph, |b, g| {
            b.iter(|| dijkstra::dijkstra(black_box(g), source, |&w| w))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_dijkstra);
criterion_main!(benches);
