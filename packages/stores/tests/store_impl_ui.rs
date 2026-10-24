#[test]
fn store_impl_rejects_non_methods() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail_store_assoc_const.rs");
    t.compile_fail("tests/ui/fail_store_assoc_type.rs");
}
