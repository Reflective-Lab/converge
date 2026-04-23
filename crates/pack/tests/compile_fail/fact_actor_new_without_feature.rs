// Contract: promotion metadata constructors are hidden from pack consumers
// unless kernel authority is explicitly enabled.

use converge_pack::{FactActor, FactActorKind};

fn main() {
    let _ = FactActor::construct("converge-engine", FactActorKind::System);
}
