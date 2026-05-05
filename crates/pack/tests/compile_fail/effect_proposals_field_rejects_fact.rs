// Contract: ContextFact and ProposedFact are distinct types with no implicit conversion.
// You cannot assign a ContextFact where a ProposedFact is expected.

use converge_pack::{ContextFact, ProposedFact};

fn check(fact: ContextFact) {
    let _: ProposedFact = fact;
}

fn main() {}
