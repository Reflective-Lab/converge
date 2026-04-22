//! EXP-001: Reference validation for optimization algorithms.
//!
//! Each test uses a published or hand-computable reference input with a known
//! correct answer. If converge disagrees with the reference, the test fails.

use converge_optimization::assignment::{AssignmentProblem, hungarian};
use converge_optimization::graph::dijkstra;
use converge_optimization::graph::flow::{
    FlowNetwork, MinCostFlowProblem, max_flow, min_cost_flow,
};
use converge_optimization::graph::matching::bipartite_matching;
use converge_optimization::knapsack::{self, KnapsackProblem};
use converge_optimization::scheduling::{Interval, SchedulingProblem, list_schedule};
use converge_optimization::setcover::{self, SetCoverProblem};

// ── Hungarian Algorithm ──────────────────────────────────────────────────────
// Reference: Kuhn 1955, "The Hungarian Method for the Assignment Problem"
// Textbook 3×3 example used in almost every OR textbook.

#[test]
fn hungarian_3x3_textbook() {
    // Classic 3×3 cost matrix from Taha "Operations Research" Ch. 5
    //        Task A  Task B  Task C
    // Agent 1:  9      2      7
    // Agent 2:  6      4      3
    // Agent 3:  5      8      1
    //
    // Optimal: Agent1→B(2), Agent2→A(6), Agent3→C(1) = 9
    // (or any assignment totaling 9)
    let problem = AssignmentProblem::from_costs(vec![vec![9, 2, 7], vec![6, 4, 3], vec![5, 8, 1]]);
    let solution = hungarian::solve(&problem).unwrap();
    assert_eq!(solution.total_cost, 9, "optimal cost for classic 3×3 is 9");
}

#[test]
fn hungarian_4x4_wikipedia() {
    // Wikipedia "Hungarian algorithm" worked example:
    //   [1, 2, 3]    → mapped to 4×4 for this test
    // Costs:
    //     [ 82, 83, 69, 92 ]
    //     [ 77, 37, 49, 92 ]
    //     [ 11, 69, 5,  86 ]
    //     [ 8,  9, 98, 23 ]
    //
    // Known optimal cost: 140 (8+37+5+92 or equivalent)
    // Actually: row3→col2(69), row0→col0(82)... let me use the standard answer.
    // Wikipedia gives: (0→2=69, 1→1=37, 2→0=11, 3→3=23) = 140
    let problem = AssignmentProblem::from_costs(vec![
        vec![82, 83, 69, 92],
        vec![77, 37, 49, 92],
        vec![11, 69, 5, 86],
        vec![8, 9, 98, 23],
    ]);
    let solution = hungarian::solve(&problem).unwrap();
    // The minimum cost is 140 per the Wikipedia worked example.
    assert_eq!(
        solution.total_cost, 140,
        "Wikipedia Hungarian example optimal = 140"
    );
}

// ── Hopcroft-Karp Bipartite Matching ──────────────────────────────────────────
// Reference: Hopcroft & Karp 1973, König's theorem

#[test]
fn hopcroft_karp_complete_bipartite_k3_3() {
    // K₃,₃: every left node connects to every right node.
    // Maximum matching = 3 (perfect matching exists).
    let edges: Vec<(usize, usize)> = (0..3).flat_map(|l| (0..3).map(move |r| (l, r))).collect();
    let matching = bipartite_matching(3, 3, &edges).unwrap();
    assert_eq!(matching.size, 3, "K₃,₃ has perfect matching of size 3");
}

#[test]
fn hopcroft_karp_augmenting_path_required() {
    // Classic example where a greedy approach fails:
    //   L0 → R0
    //   L0 → R1
    //   L1 → R0
    //
    // Greedy might match L0→R0, leaving L1 unmatched.
    // Hopcroft-Karp finds: L0→R1, L1→R0 (size 2).
    let edges = vec![(0, 0), (0, 1), (1, 0)];
    let matching = bipartite_matching(2, 2, &edges).unwrap();
    assert_eq!(
        matching.size, 2,
        "augmenting path yields maximum matching of 2"
    );
}

#[test]
fn hopcroft_karp_konigs_theorem_verification() {
    // König's theorem: in bipartite graphs, max matching = min vertex cover.
    // Star graph: L0 connects to R0, R1, R2, R3.
    //   L1 connects to R4.
    // Max matching = 2 (L0→any R, L1→R4).
    // Min vertex cover = 2 (L0, L1).
    let edges = vec![(0, 0), (0, 1), (0, 2), (0, 3), (1, 4)];
    let matching = bipartite_matching(2, 5, &edges).unwrap();
    assert_eq!(
        matching.size, 2,
        "star graph: max matching = min vertex cover = 2"
    );
}

// ── 0-1 Knapsack (Dynamic Programming) ──────────────────────────────────────
// Reference: Martello & Toth "Knapsack Problems", small instances

#[test]
fn knapsack_textbook_5_items() {
    // Classic textbook instance (Cormen CLRS Ch. 16 variant):
    //   Item:    1    2    3    4    5
    //   Weight:  2    3    4    5    9
    //   Value:   3    4    5    8   10
    //   Capacity: 20
    //
    // Optimal: take all items (total weight = 23 > 20, so not all).
    // Take items 1,2,3,4: weight=14, value=20. Room for item 5? 14+9=23>20. No.
    // Take items 2,3,4: weight=12, value=17. Add item 5: 12+9=21>20. No. Add item 1: 12+2=14, value=21.
    // Take items 1,2,4,5: weight=2+3+5+9=19, value=3+4+8+10=25. Fits!
    // Take items 1,3,4,5: weight=2+4+5+9=20, value=3+5+8+10=26. Fits exactly!
    // That's the optimum: 26.
    let problem = KnapsackProblem::new(vec![2, 3, 4, 5, 9], vec![3, 4, 5, 8, 10], 20).unwrap();
    let solution = knapsack::solve(&problem).unwrap();
    assert_eq!(
        solution.total_value, 26,
        "CLRS-variant 5-item knapsack optimal = 26"
    );
    assert!(solution.total_weight <= 20, "must fit in capacity");
}

#[test]
fn knapsack_rosetta_code_example() {
    // Rosetta Code "0-1 Knapsack" — widely used reference:
    //   Capacity: 400
    //   Items: (weight, value)
    //   map     (9, 150)
    //   compass (13, 35)
    //   water   (153, 200)
    //   sandwich(50, 160)
    //   glucose (15, 60)
    //   tin     (68, 45)
    //   banana  (27, 60)
    //   apple   (39, 40)
    //   cheese  (23, 30)
    //   beer    (52, 10)
    //   cream   (11, 70)
    //   camera  (32, 30)
    //
    // Hand-verified optimal for these 12 items: 835 value, 372 weight.
    // Take all items except tin(68,45) and beer(52,10) = 835 value, 372 weight.
    // Remaining capacity 28 cannot fit tin or beer.
    let problem = KnapsackProblem::new(
        vec![9, 13, 153, 50, 15, 68, 27, 39, 23, 52, 11, 32],
        vec![150, 35, 200, 160, 60, 45, 60, 40, 30, 10, 70, 30],
        400,
    )
    .unwrap();
    let solution = knapsack::solve(&problem).unwrap();
    assert_eq!(
        solution.total_value, 835,
        "12-item knapsack optimal = 835 (take all except tin and beer)"
    );
    assert!(solution.total_weight <= 400);
}

// ── Dijkstra's Shortest Path ──────────────────────────────────────────────────
// Reference: Dijkstra 1959, textbook weighted digraph

#[test]
fn dijkstra_textbook_graph() {
    // Textbook graph (Sedgewick "Algorithms" style):
    //   0 →(7)→ 1 →(10)→ 3
    //   0 →(9)→ 2 →(1)→ 1
    //   2 →(2)→ 3
    //
    // Shortest 0→3: 0→2(9)→3(2) = 11 (NOT 0→1(7)→3(10)=17)
    use petgraph::graph::DiGraph;

    let mut g = DiGraph::<(), i64>::new();
    let n0 = g.add_node(());
    let n1 = g.add_node(());
    let n2 = g.add_node(());
    let n3 = g.add_node(());

    g.add_edge(n0, n1, 7);
    g.add_edge(n0, n2, 9);
    g.add_edge(n1, n3, 10);
    g.add_edge(n2, n1, 1);
    g.add_edge(n2, n3, 2);

    let path = dijkstra::shortest_path(&g, n0, n3, |e| *e)
        .unwrap()
        .unwrap();
    assert_eq!(path.cost, 11, "shortest 0→3 = 11 via 0→2→3");
    // Full path must be 0→2→3 (not the direct-hop stub that was previously returned).
    assert_eq!(path.nodes, vec![n0, n2, n3], "path must traverse 0→2→3");

    // Also verify all-pairs distances from source.
    let distances = dijkstra::dijkstra(&g, n0, |e| *e).unwrap();
    assert_eq!(distances[&n1], 7, "0→1 = 7 (direct)");
    assert_eq!(distances[&n2], 9, "0→2 = 9 (direct)");
    assert_eq!(distances[&n3], 11, "0→3 = 11 (via 2)");
}

// ── Max Flow ──────────────────────────────────────────────────────────────────
// Reference: Ford & Fulkerson, standard textbook network

#[test]
fn max_flow_textbook_network() {
    // Classic 4-node max flow (Sedgewick):
    //   s(0) →(10)→ 1 →(10)→ t(3)
    //   s(0) →(10)→ 2 →(10)→ t(3)
    //   1    →(1)→  2
    //
    // Max flow = 20 (both s→1→t and s→2→t saturated, cross-edge immaterial)
    let mut net = FlowNetwork::new(4);
    net.add_edge(0, 1, 10, 0);
    net.add_edge(0, 2, 10, 0);
    net.add_edge(1, 3, 10, 0);
    net.add_edge(2, 3, 10, 0);
    net.add_edge(1, 2, 1, 0);

    let result = max_flow(&net, 0, 3).unwrap();
    assert_eq!(result.max_flow, 20, "parallel paths: max flow = 20");
}

// ── Min-Cost Flow ─────────────────────────────────────────────────────────────
// Reference: Ahuja, Magnanti, Orlin "Network Flows"

#[test]
fn min_cost_flow_simple_network() {
    // Network: 2 parallel paths from source (0) to sink (3), demand = 4
    //   0 →(cap=3, cost=1)→ 1 →(cap=3, cost=1)→ 3
    //   0 →(cap=3, cost=5)→ 2 →(cap=3, cost=5)→ 3
    //
    // Cheapest: send 3 via path 0→1→3 (cost 3×2=6), send 1 via 0→2→3 (cost 1×10=10)
    // Total cost = 6+10 = 16
    let mut net = FlowNetwork::new(4);
    net.add_edge(0, 1, 3, 1);
    net.add_edge(1, 3, 3, 1);
    net.add_edge(0, 2, 3, 5);
    net.add_edge(2, 3, 3, 5);

    let problem = MinCostFlowProblem::source_sink(net, 0, 3, 4).unwrap();
    let result = min_cost_flow(&problem).unwrap();
    assert_eq!(result.flow, 4, "total flow = demand = 4");
    assert_eq!(result.cost, 16, "min cost for 4 units = 16");
}

// ── Set Cover (Greedy) ───────────────────────────────────────────────────────
// Reference: Chvátal 1979 — greedy achieves O(ln n) approximation

#[test]
fn set_cover_textbook_instance() {
    // Universe = {0, 1, 2, 3, 4}
    // Sets:
    //   S0 = {0, 1, 2}     cost = 3
    //   S1 = {2, 3}         cost = 2
    //   S2 = {3, 4}         cost = 2
    //   S3 = {0, 1, 2, 3, 4} cost = 6
    //
    // Optimal: S0 + S2 = cost 5 (covers all).
    // Greedy by cost-effectiveness: S0 covers 3 elems for 3 (1.0/elem),
    //   S1 covers 1 new for 2 (2.0/elem), S2 covers 1 new for 2 (2.0/elem).
    // Greedy picks S0(3), then S1(2) or S2(2) for remaining, total = 3+2+2=7.
    // But S0 covers {0,1,2}, then S2 covers {3,4} → total 5 (greedy CAN find this).
    // Actually greedy: S0 = 3/3 = 1.0, S1 = 2/2 = 1.0, S2 = 2/2 = 1.0, S3 = 6/5 = 1.2.
    // Picks S0 (arbitrary among equal). Remaining: {3,4}. S2 covers both for 2.0/2=1.0.
    // Greedy total: 3 + 2 = 5 = optimal!
    let problem = SetCoverProblem::new(
        5,
        vec![
            (3, vec![0, 1, 2]),
            (2, vec![2, 3]),
            (2, vec![3, 4]),
            (6, vec![0, 1, 2, 3, 4]),
        ],
    )
    .unwrap();
    let solution = setcover::solve(&problem).unwrap();
    // Greedy guarantee: cost ≤ H(n) × OPT ≤ ln(5)+1 × 5 ≈ 12.6
    assert!(
        solution.total_cost <= 13,
        "greedy cost {} must be within O(ln n) of optimal 5",
        solution.total_cost
    );
    // In this case, greedy should actually find the optimal.
    assert_eq!(
        solution.total_cost, 5,
        "greedy finds optimal for this instance"
    );
}

// ── Scheduling ───────────────────────────────────────────────────────────────
// Reference: list scheduling heuristic, hand-verifiable

#[test]
fn disjunctive_scheduling_hand_verifiable() {
    // 3 tasks on 1 machine (disjunctive = no overlap):
    //   Task 0: earliest=0, latest=10, duration=3
    //   Task 1: earliest=0, latest=10, duration=2
    //   Task 2: earliest=0, latest=10, duration=4
    //
    // List scheduling (EDD): schedule in order, no overlap.
    // Optimal makespan: 3+2+4 = 9
    let intervals = vec![
        Interval::new(0, 0, 10, 3),
        Interval::new(1, 0, 10, 2),
        Interval::new(2, 0, 10, 4),
    ];
    let problem = SchedulingProblem::disjunctive(intervals);
    let solution = list_schedule(&problem).unwrap();
    assert_eq!(
        solution.makespan, 9,
        "3 sequential tasks: makespan = sum of durations = 9"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// LARGER INSTANCES — stress the implementation beyond trivial cases
// ════════════════════════════════════════════════════════════════════════════

// ── Hungarian 8×8 ───────────────────────────────────────────────────────────
// Constructed so each row has a unique column minimum, and the sum of row
// minimums equals the optimal. This is provably optimal because:
//   optimal ≥ Σ(row_min) (lower bound), and we exhibit a feasible assignment
//   achieving that bound.

#[test]
fn hungarian_8x8_planted_optimal() {
    // Row minimums at distinct columns: (0→4)=3, (1→7)=2, (2→1)=4, (3→5)=1,
    // (4→0)=5, (5→3)=2, (6→6)=3, (7→2)=4. Sum = 24.
    #[rustfmt::skip]
    let costs = vec![
        vec![15, 12,  9,  8,  3, 14, 10, 11],  // min=3 at col 4
        vec![11,  7, 13, 12,  8,  6, 15,  2],  // min=2 at col 7
        vec![14,  4, 12, 16,  9, 11,  7, 13],  // min=4 at col 1
        vec![10, 15, 11,  8, 13,  1,  9, 14],  // min=1 at col 5
        vec![ 5, 11, 16, 14,  7, 10, 12,  9],  // min=5 at col 0
        vec![12,  9,  7,  2, 11, 15, 13,  8],  // min=2 at col 3
        vec![ 8, 13, 10, 11, 14,  9,  3, 16],  // min=3 at col 6
        vec![13, 10,  4, 15, 12,  8, 11,  6],  // min=4 at col 2
    ];
    let problem = AssignmentProblem::from_costs(costs);
    let solution = hungarian::solve(&problem).unwrap();
    assert_eq!(solution.total_cost, 24, "8×8 planted optimal = 24");
}

// ── Hopcroft-Karp 6×6 ──────────────────────────────────────────────────────
// Sparse bipartite graph requiring multiple augmentation rounds.

#[test]
fn hopcroft_karp_6x6_sparse() {
    // Each left vertex has exactly 2 neighbors. Greedy can get stuck.
    // L0→R0,R1  L1→R1,R2  L2→R2,R3  L3→R3,R4  L4→R4,R5  L5→R5,R0
    // This is a cycle structure. Perfect matching = 6.
    // One solution: L0→R0, L1→R1, L2→R2, L3→R3, L4→R4, L5→R5.
    let edges = vec![
        (0, 0),
        (0, 1),
        (1, 1),
        (1, 2),
        (2, 2),
        (2, 3),
        (3, 3),
        (3, 4),
        (4, 4),
        (4, 5),
        (5, 5),
        (5, 0),
    ];
    let matching = bipartite_matching(6, 6, &edges).unwrap();
    assert_eq!(matching.size, 6, "6×6 cycle structure has perfect matching");
}

// ── Knapsack 15 items ───────────────────────────────────────────────────────
// Greedy-by-ratio gives suboptimal answer. DP must explore combinations.

#[test]
fn knapsack_15_items_greedy_suboptimal() {
    // Items sorted by value/weight ratio (descending):
    //   #0  w=10 v=60  (6.0)   #1  w=5  v=28  (5.6)
    //   #2  w=15 v=75  (5.0)   #3  w=8  v=36  (4.5)
    //   #4  w=12 v=50  (4.17)  #5  w=3  v=12  (4.0)
    //   #6  w=20 v=70  (3.5)   #7  w=7  v=22  (3.14)
    //   #8  w=2  v=6   (3.0)   #9  w=11 v=30  (2.73)
    //   #10 w=18 v=45  (2.5)   #11 w=6  v=14  (2.33)
    //   #12 w=14 v=30  (2.14)  #13 w=9  v=18  (2.0)
    //   #14 w=4  v=7   (1.75)
    //
    // Capacity: 50
    // Greedy-by-ratio: {0,1,2,3,5,8,7} w=50 v=239
    // Optimal DP:      {0,1,2,3,4}     w=50 v=249
    let problem = KnapsackProblem::new(
        vec![10, 5, 15, 8, 12, 3, 20, 7, 2, 11, 18, 6, 14, 9, 4],
        vec![60, 28, 75, 36, 50, 12, 70, 22, 6, 30, 45, 14, 30, 18, 7],
        50,
    )
    .unwrap();
    let solution = knapsack::solve(&problem).unwrap();
    assert_eq!(
        solution.total_value, 249,
        "15-item knapsack: DP beats greedy (249 > 239)"
    );
    assert!(solution.total_weight <= 50);
}

// ── Dijkstra 10-node graph ──────────────────────────────────────────────────
// Shortest path to target traverses 4 intermediate nodes.

#[test]
fn dijkstra_10_node_multi_hop() {
    use petgraph::graph::DiGraph;

    // Graph with 10 nodes. Shortest 0→9 = 0→2→3→7→9 (cost 7).
    //   0→1(4)  0→2(2)  0→4(7)
    //   2→5(1)  2→3(3)
    //   3→6(2)  3→7(1)
    //   4→7(1)
    //   5→8(3)
    //   6→8(1)  6→9(4)
    //   7→9(1)
    //   8→9(2)
    let mut g = DiGraph::<(), i64>::new();
    let nodes: Vec<_> = (0..10).map(|_| g.add_node(())).collect();

    let edges = [
        (0, 1, 4),
        (0, 2, 2),
        (0, 4, 7),
        (2, 5, 1),
        (2, 3, 3),
        (3, 6, 2),
        (3, 7, 1),
        (4, 7, 1),
        (5, 8, 3),
        (6, 8, 1),
        (6, 9, 4),
        (7, 9, 1),
        (8, 9, 2),
    ];
    for (from, to, weight) in edges {
        g.add_edge(nodes[from], nodes[to], weight);
    }

    let distances = dijkstra::dijkstra(&g, nodes[0], |e| *e).unwrap();

    // Hand-computed shortest distances from 0:
    // 0→1=4, 0→2=2, 0→3=5, 0→4=7, 0→5=3, 0→6=7, 0→7=6, 0→8=6, 0→9=7
    assert_eq!(distances[&nodes[1]], 4);
    assert_eq!(distances[&nodes[2]], 2);
    assert_eq!(distances[&nodes[3]], 5, "0→2→3 = 2+3 = 5");
    assert_eq!(distances[&nodes[4]], 7);
    assert_eq!(distances[&nodes[5]], 3, "0→2→5 = 2+1 = 3");
    assert_eq!(distances[&nodes[6]], 7, "0→2→3→6 = 2+3+2 = 7");
    assert_eq!(distances[&nodes[7]], 6, "0→2→3→7 = 2+3+1 = 6");
    assert_eq!(distances[&nodes[8]], 6, "0→2→5→8 = 2+1+3 = 6");
    assert_eq!(distances[&nodes[9]], 7, "0→2→3→7→9 = 2+3+1+1 = 7");
}

// ── Max Flow with bottleneck ────────────────────────────────────────────────
// Max flow < source capacity (intermediate bottleneck).

#[test]
fn max_flow_bottleneck_network() {
    // 6 nodes: s=0, a=1, b=2, c=3, d=4, t=5
    //   s→a(10)  s→b(10)
    //   a→c(4)   a→d(7)
    //   b→c(5)   b→d(3)
    //   c→t(6)   d→t(8)
    //
    // Source can push 20, but c→t(6)+d→t(8)=14 is the bottleneck.
    // Max flow = 14.
    let mut net = FlowNetwork::new(6);
    net.add_edge(0, 1, 10, 0); // s→a
    net.add_edge(0, 2, 10, 0); // s→b
    net.add_edge(1, 3, 4, 0); // a→c
    net.add_edge(1, 4, 7, 0); // a→d
    net.add_edge(2, 3, 5, 0); // b→c
    net.add_edge(2, 4, 3, 0); // b→d
    net.add_edge(3, 5, 6, 0); // c→t
    net.add_edge(4, 5, 8, 0); // d→t

    let result = max_flow(&net, 0, 5).unwrap();
    assert_eq!(
        result.max_flow, 14,
        "bottleneck at sink layer: max flow = 14"
    );
}

// ── Min-Cost Flow with 3 competing paths ────────────────────────────────────
// Algorithm must fill cheapest path first, then next cheapest.

#[test]
fn min_cost_flow_three_paths() {
    // 5 nodes: s=0, a=1, b=2, c=3, t=4
    // Path s→a→t: cap=4, cost=1+1=2 per unit (cheapest)
    // Path s→c→t: cap=4, cost=2+2=4 per unit (medium)
    // Path s→b→t: cap=4, cost=3+3=6 per unit (expensive)
    //
    // Demand = 10. Fill: 4×path_a(8) + 4×path_c(16) + 2×path_b(12) = 36.
    let mut net = FlowNetwork::new(5);
    net.add_edge(0, 1, 4, 1); // s→a
    net.add_edge(1, 4, 4, 1); // a→t
    net.add_edge(0, 2, 4, 3); // s→b
    net.add_edge(2, 4, 4, 3); // b→t
    net.add_edge(0, 3, 4, 2); // s→c
    net.add_edge(3, 4, 4, 2); // c→t

    let problem = MinCostFlowProblem::source_sink(net, 0, 4, 10).unwrap();
    let result = min_cost_flow(&problem).unwrap();
    assert_eq!(result.flow, 10);
    assert_eq!(result.cost, 36, "3-path min cost: 4×2 + 4×4 + 2×6 = 36");
}

// ── Set Cover 10 elements ───────────────────────────────────────────────────
// Larger universe with overlapping sets.

#[test]
fn set_cover_10_elements() {
    // Universe = {0..9}, 6 sets.
    // S0={0,1,2,3}     cost=4   (1.0/elem)
    // S1={2,3,4,5}     cost=4   (1.0/elem)
    // S2={4,5,6,7}     cost=4   (1.0/elem)
    // S3={6,7,8,9}     cost=4   (1.0/elem)
    // S4={0,1,2,3,4,5} cost=7   (1.17/elem)
    // S5={0..9}        cost=12  (1.2/elem)
    //
    // Optimal: S0+S2+S3 = 12 (or S0+S1+S3 if they cover all).
    // S0={0,1,2,3}, S2={4,5,6,7}, S3={6,7,8,9} → covers {0..9}. Cost=12. ✓
    // Greedy: all have 1.0/elem. Picks S0(4 new), then S1 or S2 (2 or 4 new).
    // After S0: remaining {4,5,6,7,8,9}. S2 covers 4 for 1.0/elem. Pick S2.
    // After S0+S2: remaining {8,9}. S3 covers 2 for 2.0/elem.
    // Greedy total = 4+4+4 = 12 = optimal.
    let problem = SetCoverProblem::new(
        10,
        vec![
            (4, vec![0, 1, 2, 3]),
            (4, vec![2, 3, 4, 5]),
            (4, vec![4, 5, 6, 7]),
            (4, vec![6, 7, 8, 9]),
            (7, vec![0, 1, 2, 3, 4, 5]),
            (12, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
        ],
    )
    .unwrap();
    let solution = setcover::solve(&problem).unwrap();
    assert!(solution.total_cost <= 12, "greedy should find cost ≤ 12");
}

// ── Scheduling with varying time windows ────────────────────────────────────
// Tasks have different earliest-start and latest-end constraints.

#[test]
fn scheduling_6_tasks_varying_windows() {
    // 6 tasks, 1 machine, all with wide windows:
    //   Task 0: [0, 30] dur=5
    //   Task 1: [0, 30] dur=3
    //   Task 2: [0, 30] dur=7
    //   Task 3: [0, 30] dur=2
    //   Task 4: [0, 30] dur=4
    //   Task 5: [0, 30] dur=3
    //
    // Makespan = 5+3+7+2+4+3 = 24
    let intervals = vec![
        Interval::new(0, 0, 30, 5),
        Interval::new(1, 0, 30, 3),
        Interval::new(2, 0, 30, 7),
        Interval::new(3, 0, 30, 2),
        Interval::new(4, 0, 30, 4),
        Interval::new(5, 0, 30, 3),
    ];
    let problem = SchedulingProblem::disjunctive(intervals);
    let solution = list_schedule(&problem).unwrap();
    assert_eq!(
        solution.makespan, 24,
        "6 tasks: makespan = sum of durations = 24"
    );
}
