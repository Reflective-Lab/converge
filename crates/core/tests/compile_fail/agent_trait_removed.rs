// Contract: The old `Agent` trait no longer exists.
// Only `Suggestor` is the valid trait for convergence participants.

fn main() {
    // Attempting to reference the removed Agent trait.
    let _: &dyn converge_core::Agent;
}
