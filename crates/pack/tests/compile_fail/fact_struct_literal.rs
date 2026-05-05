// Prove: ContextFact fields are private — struct literal construction is impossible.
// This file must FAIL to compile.

use converge_pack::{ContextFact, ContextKey};

fn main() {
    let _fact = ContextFact {
        key: ContextKey::Seeds,
        id: "test-id".to_string(),
        content: "test-content".to_string(),
    };
}
