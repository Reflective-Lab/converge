// Contract: only converge-core itself may promote facts into ContextState.
// External embedders can stage proposals, but cannot call add_fact().

use converge_core::{ContextState, Fact};

fn try_promote_directly(context: &mut ContextState, fact: Fact) {
    let _ = context.add_fact(fact);
}

fn main() {}
