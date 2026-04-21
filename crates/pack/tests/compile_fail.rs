#[test]
fn compile_fail_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/context_key_exhaustive.rs");
    t.compile_fail("tests/compile_fail/effect_no_facts_field.rs");
    t.compile_fail("tests/compile_fail/effect_with_fact_removed.rs");
    t.compile_fail("tests/compile_fail/effect_proposals_field_rejects_fact.rs");
    t.compile_fail("tests/compile_fail/fact_new_without_feature.rs");
    t.compile_fail("tests/compile_fail/fact_struct_literal.rs");
    t.compile_fail("tests/compile_fail/proposed_fact_bypass_confidence.rs");
    t.compile_fail("tests/compile_fail/proposed_fact_struct_literal.rs");
    t.compile_fail("tests/compile_fail/proposed_fact_try_into_fact.rs");
    t.compile_fail("tests/compile_fail/suggestor_direct_promote.rs");

    // These cases only prove the consumer boundary when converge-pack itself is
    // built without kernel-authority. In workspace-wide runs the feature can be
    // enabled transitively by converge-core, which would make these fixtures
    // compile successfully for the wrong reason.
    if !cfg!(feature = "kernel-authority") {
        t.compile_fail("tests/compile_fail/fact_actor_new_without_feature.rs");
        t.compile_fail("tests/compile_fail/kernel_authority_module_hidden.rs");
    }
}
