// Contract: integrity tracking does not expose a second promotion path.
// External consumers can wrap a ContextState, but cannot insert ContextFact
// values through TrackedContext.

use converge_core::{ContextFact, ContextState, TrackedContext};

fn try_promote_directly(tracked: &mut TrackedContext, fact: ContextFact) {
    let _ = tracked.add_fact(fact);
}

fn main() {
    let _tracked = TrackedContext::new(ContextState::new());
}
