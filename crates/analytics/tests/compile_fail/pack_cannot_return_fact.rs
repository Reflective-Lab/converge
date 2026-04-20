// Prove: A Pack's solve() returns PackSolveResult (which contains ProposedPlan),
// not a Fact. You cannot bypass the promotion gate by returning a Fact from solve().

use converge_optimization::packs::Pack;
use converge_pack::Fact;

struct EvilPack;

impl Pack for EvilPack {
    fn name(&self) -> &'static str { "evil" }
    fn version(&self) -> &'static str { "1.0.0" }

    fn validate_inputs(&self, _: &serde_json::Value) -> converge_optimization::Result<()> {
        Ok(())
    }

    fn invariants(&self) -> &[converge_optimization::packs::InvariantDef] {
        &[]
    }

    // Wrong return type: Fact instead of PackSolveResult
    fn solve(&self, _spec: &converge_optimization::gate::ProblemSpec) -> converge_optimization::Result<Fact> {
        todo!()
    }

    fn check_invariants(&self, _: &converge_optimization::gate::ProposedPlan) -> converge_optimization::Result<Vec<converge_optimization::packs::InvariantResult>> {
        Ok(vec![])
    }

    fn evaluate_gate(&self, _: &converge_optimization::gate::ProposedPlan, _: &[converge_optimization::packs::InvariantResult]) -> converge_optimization::gate::PromotionGate {
        converge_optimization::gate::PromotionGate::auto_promote("ok")
    }
}

fn main() {}
