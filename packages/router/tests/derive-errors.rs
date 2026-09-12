#[test]
fn derive_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/derive-errors/nest-missing-field.rs");
    t.compile_fail("tests/derive-errors/child-dynamic-prefix.rs");
    t.compile_fail("tests/derive-errors/child-catchall-prefix.rs");
}
