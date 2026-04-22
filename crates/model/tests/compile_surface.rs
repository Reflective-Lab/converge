#[test]
fn compile_formation_semantic_surface() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass_formation_surface.rs");
}
