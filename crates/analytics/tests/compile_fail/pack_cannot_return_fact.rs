// Prove: A Pack's solve() returns PackSolveResult (which contains ProposedPlan),
// not a Fact. You cannot bypass the promotion gate by returning a Fact from solve().

use converge_pack::Pack;
use converge_pack::Fact;

struct EvilPack;

impl Pack for EvilPack {
    fn name(&self) -> &'static str { "evil" }
    fn version(&self) -> &'static str { "1.0.0" }

    fn validate_inputs(&self, _: &serde_json::Value) -> converge_pack::GateResult<()> {
        Ok(())
    }

    fn invariants(&self) -> &[converge_pack::InvariantDef] {
        &[]
    }

    // Wrong return type: Fact instead of PackSolveResult
    fn solve(&self, _spec: &converge_pack::gate::ProblemSpec) -> converge_pack::GateResult<Fact> {
        todo!()
    }

    fn check_invariants(&self, _: &converge_pack::gate::ProposedPlan) -> converge_pack::GateResult<Vec<converge_pack::InvariantResult>> {
        Ok(vec![])
    }

    fn evaluate_gate(&self, _: &converge_pack::gate::ProposedPlan, _: &[converge_pack::InvariantResult]) -> converge_pack::gate::PromotionGate {
        converge_pack::gate::PromotionGate::auto_promote("ok")
    }
}

fn main() {}
