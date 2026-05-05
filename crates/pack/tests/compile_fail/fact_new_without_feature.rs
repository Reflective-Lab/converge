// Prove: external code cannot create authoritative facts.
// The old kernel-authority constructor is gone. ContextFact is a read-only
// projection surface, not a downstream promotion API.
// This file must FAIL to compile.

use converge_pack::{ContextFact, ContextKey};

fn main() {
    let _fact = ContextFact::construct(ContextKey::Seeds, "test-id", "test-content");
}
