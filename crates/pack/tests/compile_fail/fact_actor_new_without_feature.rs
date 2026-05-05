// Contract: promotion metadata authority constructors are not public pack API.

use converge_pack::{FactActor, FactActorKind};

fn main() {
    let _ = FactActor::construct("converge-engine", FactActorKind::System);
}
