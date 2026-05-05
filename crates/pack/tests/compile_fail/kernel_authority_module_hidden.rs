// Contract: kernel-authority helpers do not exist in the public pack API.

use converge_pack::{fact::kernel_authority, ContextKey};

fn main() {
    let _ = kernel_authority::new_fact(ContextKey::Seeds, "seed-1", "seed content");
}
