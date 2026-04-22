#[test]
fn compile_grouped_formation_surface() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass_formation_surface.rs");
}
