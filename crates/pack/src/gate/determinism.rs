//! Determinism specification for reproducible results

use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Determinism specification for reproducible results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeterminismSpec {
    /// Random seed for reproducibility
    pub seed: u64,
    /// Tie-breaking strategy
    pub tie_break: TieBreakStrategy,
    /// Whether to enforce stable sorting
    pub stable_sort: bool,
    /// Version of determinism rules (for forward compatibility)
    pub rules_version: String,
}

impl Default for DeterminismSpec {
    fn default() -> Self {
        Self {
            seed: 0,
            tie_break: TieBreakStrategy::LexicographicFirst,
            stable_sort: true,
            rules_version: "v1".to_string(),
        }
    }
}

impl DeterminismSpec {
    /// Create with explicit seed
    pub fn with_seed(seed: u64) -> Self {
        Self {
            seed,
            ..Default::default()
        }
    }

    /// Create with seed and tie-break strategy
    pub fn new(seed: u64, tie_break: TieBreakStrategy) -> Self {
        Self {
            seed,
            tie_break,
            stable_sort: true,
            rules_version: "v1".to_string(),
        }
    }

    /// Generate a sub-seed for a specific phase
    ///
    /// This ensures different phases get different but deterministic seeds
    /// derived from the main seed.
    pub fn sub_seed(&self, phase: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.seed.hash(&mut hasher);
        phase.hash(&mut hasher);
        hasher.finish()
    }

    /// Generate a sub-seed for a numbered iteration
    pub fn iter_seed(&self, iteration: usize) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.seed.hash(&mut hasher);
        iteration.hash(&mut hasher);
        hasher.finish()
    }
}

/// Strategy for breaking ties when multiple solutions are equally good
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TieBreakStrategy {
    /// Choose first in lexicographic order of identifiers
    LexicographicFirst,
    /// Choose last in lexicographic order
    LexicographicLast,
    /// Choose by smallest index/id
    SmallestIndex,
    /// Choose by largest index/id
    LargestIndex,
    /// Use seeded random selection
    SeededRandom,
}

impl TieBreakStrategy {
    /// Apply tie-breaking to select from candidates
    pub fn select<T: Ord + Clone>(&self, candidates: &[T], seed: u64) -> Option<T> {
        if candidates.is_empty() {
            return None;
        }
        match self {
            Self::LexicographicFirst | Self::SmallestIndex => candidates.iter().min().cloned(),
            Self::LexicographicLast | Self::LargestIndex => candidates.iter().max().cloned(),
            Self::SeededRandom => {
                let idx = (seed as usize) % candidates.len();
                Some(candidates[idx].clone())
            }
        }
    }

    /// Apply tie-breaking with a custom comparator
    pub fn select_by<'a, T, F>(&self, candidates: &'a [T], seed: u64, compare: F) -> Option<&'a T>
    where
        F: Fn(&T, &T) -> std::cmp::Ordering,
    {
        if candidates.is_empty() {
            return None;
        }
        match self {
            Self::LexicographicFirst | Self::SmallestIndex => {
                candidates.iter().min_by(|a, b| compare(a, b))
            }
            Self::LexicographicLast | Self::LargestIndex => {
                candidates.iter().max_by(|a, b| compare(a, b))
            }
            Self::SeededRandom => {
                let idx = (seed as usize) % candidates.len();
                Some(&candidates[idx])
            }
        }
    }

    /// Sort candidates according to this strategy
    pub fn sort<T: Ord>(&self, candidates: &mut [T]) {
        match self {
            Self::LexicographicFirst | Self::SmallestIndex => candidates.sort(),
            Self::LexicographicLast | Self::LargestIndex => {
                candidates.sort();
                candidates.reverse();
            }
            Self::SeededRandom => {
                // For random, we don't reorder - caller should use select
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_determinism() {
        let spec = DeterminismSpec::default();
        assert_eq!(spec.seed, 0);
        assert_eq!(spec.tie_break, TieBreakStrategy::LexicographicFirst);
        assert!(spec.stable_sort);
    }

    #[test]
    fn test_sub_seed_consistency() {
        let spec = DeterminismSpec::with_seed(12345);

        let seed1 = spec.sub_seed("phase1");
        let seed2 = spec.sub_seed("phase1");
        assert_eq!(seed1, seed2);

        let seed3 = spec.sub_seed("phase2");
        assert_ne!(seed1, seed3);
    }

    #[test]
    fn test_tie_break_lexicographic() {
        let candidates = vec!["charlie", "alice", "bob"];

        let first = TieBreakStrategy::LexicographicFirst.select(&candidates, 0);
        assert_eq!(first, Some("alice"));

        let last = TieBreakStrategy::LexicographicLast.select(&candidates, 0);
        assert_eq!(last, Some("charlie"));
    }

    #[test]
    fn test_tie_break_seeded() {
        let candidates = vec![1, 2, 3, 4, 5];

        let sel1 = TieBreakStrategy::SeededRandom.select(&candidates, 42);
        let sel2 = TieBreakStrategy::SeededRandom.select(&candidates, 42);
        assert_eq!(sel1, sel2);

        let sel3 = TieBreakStrategy::SeededRandom.select(&candidates, 43);
        let _ = sel3;
    }

    #[test]
    fn test_tie_break_empty() {
        let empty: Vec<i32> = vec![];
        assert_eq!(TieBreakStrategy::LexicographicFirst.select(&empty, 0), None);
    }
}
