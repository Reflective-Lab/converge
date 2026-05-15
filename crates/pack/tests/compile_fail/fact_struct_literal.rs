// Prove: ContextFact fields are private — struct literal construction is impossible.
// This file must FAIL to compile.
#![allow(unreachable_code)]

use converge_pack::{ContextFact, ContextKey};

fn main() {
    let _fact = ContextFact {
        key: ContextKey::Seeds,
        id: "test-id".into(),
        payload: unreachable!(),
        promotion_record: unreachable!(),
        created_at: unreachable!(),
    };
}
