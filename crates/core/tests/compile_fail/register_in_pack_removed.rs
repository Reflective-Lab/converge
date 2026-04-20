// Contract: The old `register_in_pack` function no longer exists.
// Only `register_suggestor_in_pack` is available.

use converge_core::Engine;

fn main() {
    let mut engine = Engine::new();
    // This method must not exist — only register_suggestor_in_pack is valid.
    engine.register_in_pack("pack-a", ());
}
