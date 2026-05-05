// Prove: converge-core re-exports a read-only ContextFact surface.
// External converge-core consumers must not get an authority constructor.

use converge_core::{ContextKey, ContextFact};

fn main() {
    let _fact = ContextFact::construct(ContextKey::Seeds, "test-id", "test-content");
}
