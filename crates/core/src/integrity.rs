// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Integrity primitives for Converge.
//!
//! This module provides cryptographic and causal ordering primitives that can
//! be used alongside the core Context and Fact types to add integrity features:
//!
//! - **Lamport Clock**: Provides causal ordering of events without wall clocks
//! - **Content Hash**: SHA-256 hash of content for tamper detection
//! - **Merkle Root**: Cryptographic commitment to a set of facts for audit verification
//!
//! # Example
//!
//! ```
//! use converge_core::integrity::{LamportClock, ContentHash, MerkleRoot};
//!
//! // Lamport clock for causal ordering
//! let mut clock = LamportClock::new();
//! let t1 = clock.tick(); // Event 1
//! let t2 = clock.tick(); // Event 2 (causally after Event 1)
//!
//! // Content hash for integrity
//! let hash = ContentHash::compute("important data");
//! assert!(hash.verify("important data"));
//!
//! // Merkle root for audit trail
//! let hashes = vec![
//!     ContentHash::compute("fact 1"),
//!     ContentHash::compute("fact 2"),
//! ];
//! let root = MerkleRoot::compute(&hashes);
//! ```

use serde::{Deserialize, Serialize};

use crate::context::{ContextFact, ContextKey, ContextState};

// ============================================================================
// Lamport Clock
// ============================================================================

/// A Lamport logical clock for causal ordering of events.
///
/// Lamport clocks provide a partial ordering of events in a distributed system
/// without requiring synchronized wall clocks. The key property is:
/// if event A happened-before event B, then `clock(A) < clock(B)`.
///
/// # Example
///
/// ```
/// use converge_core::integrity::LamportClock;
///
/// let mut clock = LamportClock::new();
/// assert_eq!(clock.time(), 0);
///
/// clock.tick();
/// assert_eq!(clock.time(), 1);
///
/// // When receiving a message with clock=5, update to max+1
/// clock.update(5);
/// assert_eq!(clock.time(), 6);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct LamportClock {
    time: u64,
}

impl LamportClock {
    /// Creates a new Lamport clock initialized to 0.
    #[must_use]
    pub const fn new() -> Self {
        Self { time: 0 }
    }

    /// Creates a Lamport clock at an already observed logical time.
    #[must_use]
    pub const fn at(time: u64) -> Self {
        Self { time }
    }

    /// Returns the current logical time.
    #[must_use]
    pub const fn time(&self) -> u64 {
        self.time
    }

    /// Increments the clock and returns the new time.
    /// Called before any local event (e.g., creating a fact).
    pub fn tick(&mut self) -> u64 {
        self.time += 1;
        self.time
    }

    /// Updates the clock based on a received timestamp.
    /// Sets clock to `max(local, received) + 1`.
    /// Called when receiving facts from another source.
    pub fn update(&mut self, received: u64) -> u64 {
        self.time = self.time.max(received) + 1;
        self.time
    }
}

// ============================================================================
// Content Hash
// ============================================================================

/// Error type for ContentHash operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentHashError {
    /// Invalid hexadecimal character.
    InvalidHex,
    /// Invalid string length.
    InvalidLength,
}

impl std::fmt::Display for ContentHashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHex => write!(f, "invalid hexadecimal character"),
            Self::InvalidLength => write!(f, "invalid string length (expected 64 hex chars)"),
        }
    }
}

impl std::error::Error for ContentHashError {}

/// A SHA-256 hash of content for integrity verification.
///
/// Content hashes enable:
/// - Tamper detection: any change to content changes the hash
/// - Efficient comparison: compare 32 bytes instead of full content
/// - Merkle tree construction: hashes combine into a tree
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
    /// Computes a SHA-256 hash of the given content.
    #[must_use]
    pub fn compute(content: &str) -> Self {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        Self(hasher.finalize().into())
    }

    /// Computes the hash of a Fact (combines key, id, and content).
    #[must_use]
    pub fn compute_fact(fact: &ContextFact) -> Self {
        let payload = fact
            .to_wire()
            .map(|wire| wire.payload.payload.to_string())
            .unwrap_or_else(|error| error.to_string());
        let combined = format!("{:?}|{}|{}", fact.key(), fact.id(), payload);
        Self::compute(&combined)
    }

    /// Combines two hashes (for Merkle tree internal nodes).
    #[must_use]
    pub fn combine(left: &Self, right: &Self) -> Self {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(left.0);
        hasher.update(right.0);
        Self(hasher.finalize().into())
    }

    /// Verifies that this hash matches the given content.
    #[must_use]
    pub fn verify(&self, content: &str) -> bool {
        *self == Self::compute(content)
    }

    /// Returns the hash as a hex string.
    #[must_use]
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Creates a `ContentHash` from a hex string.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not valid hex or wrong length.
    pub fn from_hex(s: &str) -> Result<Self, ContentHashError> {
        if s.len() != 64 {
            return Err(ContentHashError::InvalidLength);
        }
        let mut result = [0u8; 32];
        for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
            let high = Self::hex_char_to_nibble(chunk[0])?;
            let low = Self::hex_char_to_nibble(chunk[1])?;
            result[i] = (high << 4) | low;
        }
        Ok(Self(result))
    }

    fn hex_char_to_nibble(c: u8) -> Result<u8, ContentHashError> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err(ContentHashError::InvalidHex),
        }
    }
}

impl std::fmt::Display for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.to_hex()[..16]) // Short form for display
    }
}

// ============================================================================
// Merkle Tree
// ============================================================================

/// A Merkle tree root hash representing the integrity of a set of facts.
///
/// The Merkle root changes if any fact is added, modified, or reordered.
/// This enables:
/// - Tamper detection: compare roots to verify integrity
/// - Efficient sync: different roots mean different states
/// - Audit proofs: prove a fact exists without revealing all facts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleRoot(pub ContentHash);

impl MerkleRoot {
    /// Computes the Merkle root from a list of content hashes.
    ///
    /// Uses a standard binary Merkle tree construction.
    /// Empty list returns a hash of empty string.
    /// Single element is combined with itself (Bitcoin-style).
    #[must_use]
    pub fn compute(hashes: &[ContentHash]) -> Self {
        if hashes.is_empty() {
            return Self(ContentHash::compute(""));
        }

        let mut current_level: Vec<ContentHash> = hashes.to_vec();

        // Keep combining until we reach a single root
        // For a single element, we still iterate once to combine it with itself
        loop {
            if current_level.len() == 1 {
                // If we started with 1 element, combine with itself
                // If we reduced to 1 element through combinations, we're done
                if hashes.len() == 1 {
                    return Self(ContentHash::combine(&current_level[0], &current_level[0]));
                }
                return Self(current_level.into_iter().next().unwrap());
            }

            let mut next_level = Vec::new();

            for chunk in current_level.chunks(2) {
                let combined = if chunk.len() == 2 {
                    ContentHash::combine(&chunk[0], &chunk[1])
                } else {
                    // Odd number: duplicate the last hash
                    ContentHash::combine(&chunk[0], &chunk[0])
                };
                next_level.push(combined);
            }

            current_level = next_level;
        }
    }

    /// Computes the Merkle root from a Context's facts.
    ///
    /// Facts are hashed in deterministic order (by key, then by position).
    #[must_use]
    pub fn from_context(ctx: &ContextState) -> Self {
        let mut all_hashes: Vec<ContentHash> = Vec::new();
        let mut keys: Vec<_> = ctx.all_keys();
        keys.sort();

        for key in keys {
            for fact in ctx.get(key) {
                all_hashes.push(ContentHash::compute_fact(fact));
            }
        }

        Self::compute(&all_hashes)
    }

    /// Returns the root hash as a hex string.
    #[must_use]
    pub fn to_hex(&self) -> String {
        self.0.to_hex()
    }
}

// ============================================================================
// Tracked Context
// ============================================================================

/// A wrapper around Context that tracks integrity metadata.
///
/// This provides optional integrity tracking without modifying the core types.
#[derive(Debug, Clone, Serialize)]
pub struct TrackedContext {
    /// The underlying context.
    pub context: ContextState,
    /// Lamport clock for causal ordering.
    pub clock: LamportClock,
    /// Cached Merkle root (invalidated on changes).
    merkle_root: Option<MerkleRoot>,
    /// Hash of each fact (by key and index).
    fact_hashes: Vec<(ContextKey, String, ContentHash)>,
}

impl TrackedContext {
    /// Creates a new tracked context wrapping an existing context.
    #[must_use]
    pub fn new(context: ContextState) -> Self {
        let mut tracked = Self {
            context,
            clock: LamportClock::new(),
            merkle_root: None,
            fact_hashes: Vec::new(),
        };
        tracked.recompute_hashes();
        tracked
    }

    /// Creates an empty tracked context.
    #[must_use]
    pub fn empty() -> Self {
        Self::new(ContextState::new())
    }

    /// Returns the current Lamport clock time.
    #[must_use]
    pub fn clock_time(&self) -> u64 {
        self.clock.time()
    }

    /// Ticks the clock and returns the new time.
    pub fn tick(&mut self) -> u64 {
        self.clock.tick()
    }

    /// Returns the Lamport time assigned to the next local context event.
    #[must_use]
    pub fn next_logical_time(&self) -> u64 {
        self.clock_time() + 1
    }

    /// Restores the tracked logical clock to an already observed time.
    pub(crate) fn set_clock_time(&mut self, time: u64) {
        self.clock = LamportClock::at(time);
    }

    /// Computes and returns the Merkle root.
    #[must_use]
    pub fn merkle_root(&mut self) -> &MerkleRoot {
        if self.merkle_root.is_none() {
            self.merkle_root = Some(MerkleRoot::from_context(&self.context));
        }
        self.merkle_root.as_ref().unwrap()
    }

    /// Verifies the integrity of all facts.
    ///
    /// Returns `true` if all recorded hashes match current fact content.
    #[must_use]
    pub fn verify_integrity(&self) -> bool {
        for (key, id, expected_hash) in &self.fact_hashes {
            if let Some(fact) = self
                .context
                .get(*key)
                .iter()
                .find(|f| f.id().as_str() == id)
            {
                if ContentHash::compute_fact(fact) != *expected_hash {
                    return false;
                }
            } else {
                return false; // Fact missing
            }
        }
        true
    }

    /// Recomputes all fact hashes.
    fn recompute_hashes(&mut self) {
        self.fact_hashes.clear();
        for key in self.context.all_keys() {
            for fact in self.context.get(key) {
                let hash = ContentHash::compute_fact(fact);
                self.fact_hashes.push((key, fact.id().to_string(), hash));
            }
        }
        self.merkle_root = None; // Invalidate cached root
    }

    /// Adds a fact to the underlying context and updates tracking.
    ///
    /// # Errors
    ///
    /// Returns an error if the fact conflicts with an existing fact.
    pub(crate) fn add_fact(
        &mut self,
        fact: ContextFact,
    ) -> Result<bool, crate::error::ConvergeError> {
        let key = fact.key();
        let id = fact.id().to_string();
        let hash = ContentHash::compute_fact(&fact);

        let changed = self.context.add_fact(fact)?;

        if changed {
            self.fact_hashes.push((key, id, hash));
            self.merkle_root = None; // Invalidate cached root
            self.clock.tick();
        }

        Ok(changed)
    }

    /// Extracts an integrity proof from the current state.
    pub fn extract_proof(&mut self) -> IntegrityProof {
        let merkle_root = self.merkle_root().clone();
        IntegrityProof {
            merkle_root,
            clock_time: self.clock_time(),
            fact_count: self.fact_hashes.len(),
        }
    }
}

impl Default for TrackedContext {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// Integrity Proof
// ============================================================================

/// Cryptographic integrity proof for a converged context.
///
/// Produced by the engine at the end of convergence. Provides:
/// - Merkle root: tamper-evident commitment to all facts
/// - Clock time: causal ordering count (number of fact-addition events)
/// - Fact count: total facts in the context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrityProof {
    /// Merkle root over all facts in deterministic order.
    pub merkle_root: MerkleRoot,
    /// Final Lamport clock time (number of tracked events).
    pub clock_time: u64,
    /// Number of facts in the context.
    pub fact_count: usize,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Lamport Clock Tests
    // =========================================================================

    #[test]
    fn lamport_clock_starts_at_zero() {
        let clock = LamportClock::new();
        assert_eq!(clock.time(), 0);
    }

    #[test]
    fn lamport_clock_tick_increments() {
        let mut clock = LamportClock::new();
        assert_eq!(clock.tick(), 1);
        assert_eq!(clock.tick(), 2);
        assert_eq!(clock.tick(), 3);
    }

    #[test]
    fn lamport_clock_update_takes_max_plus_one() {
        let mut clock = LamportClock::new();
        clock.tick(); // 1
        clock.tick(); // 2

        // Received message with clock=5, should become 6
        assert_eq!(clock.update(5), 6);

        // Received message with clock=3, should become 7 (max(6,3)+1)
        assert_eq!(clock.update(3), 7);
    }

    // =========================================================================
    // Content Hash Tests
    // =========================================================================

    #[test]

    fn content_hash_is_deterministic() {
        let h1 = ContentHash::compute("hello world");
        let h2 = ContentHash::compute("hello world");
        assert_eq!(h1, h2);
    }

    #[test]

    fn content_hash_changes_with_content() {
        let h1 = ContentHash::compute("hello");
        let h2 = ContentHash::compute("world");
        assert_ne!(h1, h2);
    }

    #[test]

    fn content_hash_verify_works() {
        let hash = ContentHash::compute("test");
        assert!(hash.verify("test"));
        assert!(!hash.verify("modified"));
    }

    #[test]

    fn content_hash_hex_roundtrip() {
        let original = ContentHash::compute("test content");
        let hex = original.to_hex();
        let restored = ContentHash::from_hex(&hex).unwrap();
        assert_eq!(original, restored);
    }

    // =========================================================================
    // Merkle Tree Tests
    // =========================================================================

    #[test]

    fn merkle_root_empty_list() {
        let root = MerkleRoot::compute(&[]);
        let empty_hash = ContentHash::compute("");
        assert_eq!(root.0, empty_hash);
    }

    #[test]

    fn merkle_root_single_element() {
        let h = ContentHash::compute("only element");
        let root = MerkleRoot::compute(std::slice::from_ref(&h));
        let expected = ContentHash::combine(&h, &h);
        assert_eq!(root.0, expected);
    }

    #[test]

    fn merkle_root_two_elements() {
        let h1 = ContentHash::compute("first");
        let h2 = ContentHash::compute("second");
        let root = MerkleRoot::compute(&[h1.clone(), h2.clone()]);
        let expected = ContentHash::combine(&h1, &h2);
        assert_eq!(root.0, expected);
    }

    #[test]

    fn merkle_root_is_deterministic() {
        let hashes = vec![
            ContentHash::compute("a"),
            ContentHash::compute("b"),
            ContentHash::compute("c"),
        ];
        let root1 = MerkleRoot::compute(&hashes);
        let root2 = MerkleRoot::compute(&hashes);
        assert_eq!(root1, root2);
    }

    #[test]

    fn merkle_root_changes_with_different_content() {
        let hashes1 = vec![ContentHash::compute("a"), ContentHash::compute("b")];
        let hashes2 = vec![ContentHash::compute("a"), ContentHash::compute("c")];
        let root1 = MerkleRoot::compute(&hashes1);
        let root2 = MerkleRoot::compute(&hashes2);
        assert_ne!(root1, root2);
    }

    // =========================================================================
    // Tracked Context Tests
    // =========================================================================

    #[test]
    fn tracked_context_starts_empty() {
        let tracked = TrackedContext::empty();
        assert_eq!(tracked.clock_time(), 0);
        assert!(!tracked.context.has(ContextKey::Seeds));
    }

    #[test]
    fn tracked_context_ticks_on_add() {
        let mut tracked = TrackedContext::empty();
        assert_eq!(tracked.clock_time(), 0);

        tracked
            .add_fact(crate::context::new_fact(ContextKey::Seeds, "s1", "seed"))
            .unwrap();
        assert_eq!(tracked.clock_time(), 1);

        tracked
            .add_fact(crate::context::new_fact(ContextKey::Seeds, "s2", "seed2"))
            .unwrap();
        assert_eq!(tracked.clock_time(), 2);
    }

    #[test]
    fn tracked_context_computes_merkle_root() {
        let mut tracked = TrackedContext::empty();
        tracked
            .add_fact(crate::context::new_fact(ContextKey::Seeds, "s1", "first"))
            .unwrap();
        tracked
            .add_fact(crate::context::new_fact(ContextKey::Seeds, "s2", "second"))
            .unwrap();

        let root1 = tracked.merkle_root().clone();

        // Adding another fact changes the root
        tracked
            .add_fact(crate::context::new_fact(ContextKey::Seeds, "s3", "third"))
            .unwrap();
        let root2 = tracked.merkle_root().clone();

        assert_ne!(root1, root2);
    }

    #[test]
    fn tracked_context_verifies_integrity() {
        let mut tracked = TrackedContext::empty();
        tracked
            .add_fact(crate::context::new_fact(ContextKey::Seeds, "s1", "test"))
            .unwrap();

        assert!(tracked.verify_integrity());
    }

    #[test]
    fn integrity_proof_serializes() {
        let mut tracked = TrackedContext::empty();
        tracked
            .add_fact(crate::context::new_fact(ContextKey::Seeds, "s1", "test"))
            .unwrap();
        let proof = tracked.extract_proof();
        assert_eq!(proof.clock_time, 1);
        assert_eq!(proof.fact_count, 1);

        let json = serde_json::to_string(&proof).unwrap();
        let deser: IntegrityProof = serde_json::from_str(&json).unwrap();
        assert_eq!(proof, deser);
    }

    #[test]
    fn integrity_proof_changes_with_different_facts() {
        let mut t1 = TrackedContext::empty();
        t1.add_fact(crate::context::new_fact(ContextKey::Seeds, "s1", "alpha"))
            .unwrap();
        let proof1 = t1.extract_proof();

        let mut t2 = TrackedContext::empty();
        t2.add_fact(crate::context::new_fact(ContextKey::Seeds, "s1", "beta"))
            .unwrap();
        let proof2 = t2.extract_proof();

        assert_ne!(proof1.merkle_root, proof2.merkle_root);
        assert_eq!(proof1.clock_time, proof2.clock_time);
        assert_eq!(proof1.fact_count, proof2.fact_count);
    }
}
