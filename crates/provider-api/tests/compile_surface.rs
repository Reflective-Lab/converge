#[test]
fn compile_provider_selection_surface() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass_provider_selection_surface.rs");
}
