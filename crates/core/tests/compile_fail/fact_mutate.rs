// Contract: ContextFact projections cannot be mutated through public fields.
// Even with a ContextFact value, the `key` field is private and cannot be assigned.
// The promotion_record and created_at fields are also private.

use converge_core::{ContextKey, ContextFact};

fn attempt_mutation(mut fact: ContextFact) {
    // Private field — cannot be assigned directly.
    fact.key = ContextKey::Seeds;
}

fn main() {}
