// Contract: ProposedFact cannot be constructed via struct literal.
// The `confidence` field is private, preventing callers from setting it directly
// and bypassing the [0.0, 1.0] invariant enforced by with_confidence().

use converge_pack::{ContextKey, ProposedFact, UnitInterval};

fn main() {
    let _pf = ProposedFact {
        key: ContextKey::Seeds,
        id: "test-id".into(),
        content: "test".to_string(),
        confidence: UnitInterval::ONE,
        provenance: "attacker".to_string(),
    };
}
