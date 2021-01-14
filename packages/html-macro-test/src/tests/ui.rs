#[test]
fn ui() {
    let t = trybuild::TestCases::new();

    let ui_tests = concat!(env!("CARGO_MANIFEST_DIR"), "/src/tests/ui/*.rs");
    t.compile_fail(ui_tests);
}
