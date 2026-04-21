// Contract: ValidatedProposal cannot be forged via struct literal.
// Its fields (proposal, report) are private — only PromotionGate::validate_proposal()
// can produce one. No external code can skip validation by constructing one directly.

use converge_core::gates::ValidatedProposal;

fn main() {
    let _vp = ValidatedProposal {
        proposal: todo!(),
        report: todo!(),
    };
}
