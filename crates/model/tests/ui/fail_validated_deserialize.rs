// Prove: Proposal<Validated> cannot be deserialized from JSON.
// Only Proposal<Draft> implements Deserialize.

use converge_model::{Proposal, Validated};

fn main() {
    let json = r#"{"id":"p-1","content":{"kind":"Claim","content":"test","structured":null,"confidence":null},"provenance":{"observation_id":"obs-1","raw_payload_ref":"0000000000000000000000000000000000000000000000000000000000000000","capture_context":{"request_params":null,"environment":{},"session_id":null,"correlation_id":null}}}"#;

    // This should fail �� Deserialize not implemented for Proposal<Validated>
    let _: Proposal<Validated> = serde_json::from_str(json).unwrap();
}
