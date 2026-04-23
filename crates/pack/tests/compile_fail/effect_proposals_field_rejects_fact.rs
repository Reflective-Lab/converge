// Contract: Fact and ProposedFact are distinct types with no implicit conversion.
// You cannot assign a Fact where a ProposedFact is expected.

use converge_pack::{Fact, ProposedFact};

fn check(fact: Fact) {
    let _: ProposedFact = fact;
}

fn main() {}
