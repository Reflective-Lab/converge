// Contract: persisted context snapshots are Converge-owned envelopes.
// Storage adapters receive snapshots from ContextState::snapshot(); they do not
// fabricate snapshot internals with struct literals.

use converge_core::{ContextSnapshot, ContextState};

fn main() {
    let existing = ContextState::new().snapshot();
    let _snapshot = ContextSnapshot {
        version: existing.version(),
        merkle_root: existing.merkle_root().clone(),
        facts: existing.facts().clone(),
        proposals: existing.proposals().clone(),
    };
}
