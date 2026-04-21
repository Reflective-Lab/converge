// Contract: kernel-authority helpers are not available to normal consumers.

use converge_pack::{fact::kernel_authority, ContextKey};

fn main() {
    let _ = kernel_authority::new_fact(ContextKey::Seeds, "seed-1", "seed content");
}
