#[test]
fn derive_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/derive-errors/nest-missing-field.rs");
}
