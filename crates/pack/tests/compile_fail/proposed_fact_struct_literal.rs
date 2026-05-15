// Contract: ProposedFact cannot be constructed via struct literal.
// The `confidence` field is private, preventing callers from setting it directly
// and bypassing the [0.0, 1.0] invariant enforced by with_confidence().
#![allow(unreachable_code)]

use converge_pack::{ContextKey, ProposedFact, UnitInterval};

fn main() {
    let _pf = ProposedFact {
        key: ContextKey::Seeds,
        id: "test-id".into(),
        payload: unreachable!(),
        confidence: UnitInterval::ONE,
        provenance: "attacker".into(),
    };
}
