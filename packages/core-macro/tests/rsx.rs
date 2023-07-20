#[test]
fn rsx() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/rsx/trailing-comma-0.rs");
}
