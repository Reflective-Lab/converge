// Prove: converge-core re-exports a read-only Fact surface.
// External converge-core consumers must not be able to construct authoritative facts.

use converge_core::{ContextKey, Fact};

fn main() {
    let _fact = Fact::construct(ContextKey::Seeds, "test-id", "test-content");
}
