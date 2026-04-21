// Prove: Proposal<Validated> cannot be constructed via the model crate.
// The from_validated() method is pub(crate) on converge-core.

use converge_model::{
    CaptureContext, ObservationId, ObservationProvenance, Proposal, ProposalId,
    TypesProposedContent, Validated,
};
use converge_core::ContentHash;
use converge_core::types::ProposedContentKind;

fn main() {
    let provenance = ObservationProvenance::new(
        ObservationId::new("obs-1"),
        ContentHash::zero(),
        CaptureContext::default(),
    );

    // This should fail — from_validated is pub(crate)
    let _validated = Proposal::<Validated>::from_validated(
        ProposalId::new("p-1"),
        TypesProposedContent::new(ProposedContentKind::Claim, "test"),
        provenance,
    );
}
