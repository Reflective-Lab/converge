// Contract: ValidationReport is the unforgeable proof that validation occurred.
// Its constructor is pub(crate) — external code cannot fabricate a validation proof
// to feed into PromotionGate::promote_to_fact().

use converge_core::gates::ValidationReport;

fn main() {
    // ValidationReport::new() is pub(crate) — this must fail.
    let _ = ValidationReport::new(todo!(), vec![], todo!());
}
