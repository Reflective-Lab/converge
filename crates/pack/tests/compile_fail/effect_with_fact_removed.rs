// Contract: AgentEffect is proposal-only.
// There must not be any helper that injects authoritative facts directly.

use converge_pack::AgentEffect;

fn main() {
    let _ = AgentEffect::with_fact("Seeds", "seed-1", "seed content");
}
