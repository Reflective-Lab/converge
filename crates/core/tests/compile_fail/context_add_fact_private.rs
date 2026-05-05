// Contract: only converge-core itself may promote facts into ContextState.
// External embedders can stage proposals, but cannot call add_fact().

use converge_core::{ContextState, ContextFact};

fn try_promote_directly(context: &mut ContextState, fact: ContextFact) {
    let _ = context.add_fact(fact);
}

fn main() {}
