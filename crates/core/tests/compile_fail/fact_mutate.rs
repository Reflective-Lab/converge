// Contract: Facts cannot be mutated after creation.
// Even with a Fact value, the `key` field is private and cannot be assigned.
// The promotion_record and created_at fields are also private.

use converge_core::{ContextKey, Fact};

fn attempt_mutation(mut fact: Fact) {
    // Private field — cannot be assigned directly.
    fact.key = ContextKey::Seeds;
}

fn main() {}
