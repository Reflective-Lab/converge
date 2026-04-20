// Contract: ContextKey is a closed enum, not a newtype.
// You cannot construct a ContextKey from an arbitrary string.

use converge_pack::ContextKey;

fn main() {
    // ContextKey is an enum — no From<&str> or From<String> exists.
    let _key: ContextKey = "arbitrary".into();
}
