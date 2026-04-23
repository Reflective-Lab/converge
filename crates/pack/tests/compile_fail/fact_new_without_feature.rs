// Prove: external code cannot create authoritative facts.
// Fact::new() is gated behind cfg(feature = "kernel-authority").
// This file must FAIL to compile.

use converge_pack::{ContextKey, Fact};

fn main() {
    let _fact = Fact::construct(ContextKey::Seeds, "test-id", "test-content");
}
