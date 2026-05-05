#[test]
fn compile_fail_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/context_key_exhaustive.rs");
    t.compile_fail("tests/compile_fail/consensus_outcome_struct_literal.rs");
    t.compile_fail("tests/compile_fail/effect_no_facts_field.rs");
    t.compile_fail("tests/compile_fail/effect_proposals_field_private.rs");
    t.compile_fail("tests/compile_fail/effect_with_fact_removed.rs");
    t.compile_fail("tests/compile_fail/effect_proposals_field_rejects_fact.rs");
    t.compile_fail("tests/compile_fail/fact_new_without_feature.rs");
    t.compile_fail("tests/compile_fail/fact_struct_literal.rs");
    t.compile_fail("tests/compile_fail/proposed_fact_bypass_confidence.rs");
    t.compile_fail("tests/compile_fail/proposed_fact_struct_literal.rs");
    t.compile_fail("tests/compile_fail/proposed_fact_try_into_fact.rs");
    t.compile_fail("tests/compile_fail/suggestor_direct_promote.rs");
    t.compile_fail("tests/compile_fail/fact_actor_new_without_feature.rs");
    t.compile_fail("tests/compile_fail/kernel_authority_module_hidden.rs");
}
