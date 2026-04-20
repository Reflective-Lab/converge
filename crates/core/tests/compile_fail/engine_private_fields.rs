// Contract: Engine internal state is not accessible from outside the crate.
// All fields are private — you must use the public API.

use converge_core::Engine;

fn main() {
    let engine = Engine::new();
    // Attempt to access private field — must fail.
    let _agents = engine.agents;
}
