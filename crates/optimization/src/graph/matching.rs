//! Graph matching algorithms — Hopcroft-Karp maximum bipartite matching.

use std::collections::VecDeque;

use crate::{Error, Result};

/// Result of a maximum bipartite matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matching {
    /// Matched pairs `(left_node, right_node)`, both 0-indexed.
    pub pairs: Vec<(usize, usize)>,
    /// Size of the matching (number of matched pairs).
    pub size: usize,
}

/// Find a maximum bipartite matching using Hopcroft-Karp.
///
/// The graph has `left_count` nodes on the left side (0..left_count) and
/// `right_count` nodes on the right side (0..right_count). `edges` is a list
/// of `(left, right)` pairs; each edge must satisfy `left < left_count` and
/// `right < right_count`.
///
/// Returns the maximum matching: the largest set of edges with no shared
/// endpoint. Runs in O(E √V) time.
pub fn bipartite_matching(
    left_count: usize,
    right_count: usize,
    edges: &[(usize, usize)],
) -> Result<Matching> {
    for &(l, r) in edges {
        if l >= left_count {
            return Err(Error::invalid_input(format!(
                "left node {l} out of range (left_count={left_count})"
            )));
        }
        if r >= right_count {
            return Err(Error::invalid_input(format!(
                "right node {r} out of range (right_count={right_count})"
            )));
        }
    }

    let mut adj: Vec<Vec<usize>> = vec![vec![]; left_count];
    for &(l, r) in edges {
        adj[l].push(r);
    }

    const NONE: usize = usize::MAX;
    let mut match_left = vec![NONE; left_count];
    let mut match_right = vec![NONE; right_count];
    let mut total = 0usize;

    loop {
        // BFS: build layered graph from free left nodes.
        let mut dist = vec![NONE; left_count];
        let mut queue = VecDeque::new();

        for l in 0..left_count {
            if match_left[l] == NONE {
                dist[l] = 0;
                queue.push_back(l);
            }
        }

        let mut found_free_right = false;
        while let Some(l) = queue.pop_front() {
            for &r in &adj[l] {
                let nl = match_right[r];
                if nl == NONE {
                    found_free_right = true;
                } else if dist[nl] == NONE {
                    dist[nl] = dist[l] + 1;
                    queue.push_back(nl);
                }
            }
        }

        if !found_free_right {
            break;
        }

        // DFS: augment along shortest paths found by BFS.
        for l in 0..left_count {
            if match_left[l] == NONE && dfs(l, &adj, &mut match_left, &mut match_right, &mut dist) {
                total += 1;
            }
        }
    }

    let pairs: Vec<(usize, usize)> = (0..left_count)
        .filter(|&l| match_left[l] != NONE)
        .map(|l| (l, match_left[l]))
        .collect();

    Ok(Matching { pairs, size: total })
}

fn dfs(
    l: usize,
    adj: &[Vec<usize>],
    match_left: &mut [usize],
    match_right: &mut [usize],
    dist: &mut [usize],
) -> bool {
    const NONE: usize = usize::MAX;
    for &r in &adj[l] {
        let nl = match_right[r];
        if nl == NONE || (dist[nl] == dist[l] + 1 && dfs(nl, adj, match_left, match_right, dist)) {
            match_left[l] = r;
            match_right[r] = l;
            return true;
        }
    }
    dist[l] = NONE; // exhaust this node so it won't be revisited in this phase
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph_produces_empty_matching() {
        let m = bipartite_matching(3, 3, &[]).unwrap();
        assert_eq!(m.size, 0);
        assert!(m.pairs.is_empty());
    }

    #[test]
    fn perfect_matching_on_complete_bipartite() {
        // K_{3,3}: every left connects to every right — maximum matching is 3.
        let edges: Vec<(usize, usize)> = (0..3).flat_map(|l| (0..3).map(move |r| (l, r))).collect();
        let m = bipartite_matching(3, 3, &edges).unwrap();
        assert_eq!(m.size, 3);
        assert_eq!(m.pairs.len(), 3);
        // Each left and right appears at most once.
        let mut lefts: Vec<usize> = m.pairs.iter().map(|&(l, _)| l).collect();
        let mut rights: Vec<usize> = m.pairs.iter().map(|&(_, r)| r).collect();
        lefts.sort_unstable();
        rights.sort_unstable();
        lefts.dedup();
        rights.dedup();
        assert_eq!(lefts.len(), 3);
        assert_eq!(rights.len(), 3);
    }

    #[test]
    fn partial_matching_when_right_side_is_smaller() {
        // 4 left nodes, 2 right nodes — maximum matching is 2.
        let edges = [(0, 0), (1, 0), (2, 1), (3, 1)];
        let m = bipartite_matching(4, 2, &edges).unwrap();
        assert_eq!(m.size, 2);
    }

    #[test]
    fn single_edge_matches() {
        let m = bipartite_matching(1, 1, &[(0, 0)]).unwrap();
        assert_eq!(m.size, 1);
        assert_eq!(m.pairs, vec![(0, 0)]);
    }

    #[test]
    fn disjoint_components_all_matched() {
        // Two disjoint pairs.
        let m = bipartite_matching(2, 2, &[(0, 0), (1, 1)]).unwrap();
        assert_eq!(m.size, 2);
    }

    #[test]
    fn augmenting_path_required() {
        // Initial greedy would match 0→0, 1→0 (fail), but Hopcroft-Karp finds 0→1, 1→0.
        let m = bipartite_matching(2, 2, &[(0, 0), (0, 1), (1, 0)]).unwrap();
        assert_eq!(m.size, 2);
    }

    #[test]
    fn out_of_range_left_node_returns_error() {
        let err = bipartite_matching(2, 2, &[(5, 0)]).unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));
    }

    #[test]
    fn out_of_range_right_node_returns_error() {
        let err = bipartite_matching(2, 2, &[(0, 5)]).unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));
    }

    #[test]
    fn no_edges_means_no_matches_regardless_of_counts() {
        let m = bipartite_matching(10, 10, &[]).unwrap();
        assert_eq!(m.size, 0);
    }

    #[test]
    fn matching_is_valid_no_shared_endpoints() {
        let edges: Vec<(usize, usize)> = (0..5).flat_map(|l| (0..5).map(move |r| (l, r))).collect();
        let m = bipartite_matching(5, 5, &edges).unwrap();
        assert_eq!(m.size, 5);
        let mut seen_left = std::collections::HashSet::new();
        let mut seen_right = std::collections::HashSet::new();
        for (l, r) in &m.pairs {
            assert!(seen_left.insert(l), "left node {l} appears twice");
            assert!(seen_right.insert(r), "right node {r} appears twice");
        }
    }
}
