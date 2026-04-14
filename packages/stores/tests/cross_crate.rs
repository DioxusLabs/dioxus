//! Cross-crate sealing tests for `#[derive(Store)]`.
//!
//! `packages/stores/tests/visibility-helper` defines a pub struct with fields
//! at every visibility tier. These `trybuild` fixtures are compiled as an
//! external downstream crate and assert that only the `pub` accessor — not the
//! `pub(crate)` or private ones — can be called from outside the defining
//! crate. The same goes for transposed-struct field access.

#[test]
fn cross_crate_visibility() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass_public_field.rs");
    t.compile_fail("tests/ui/fail_crate_field.rs");
    t.compile_fail("tests/ui/fail_private_field.rs");
    t.compile_fail("tests/ui/fail_transposed_private_field.rs");
}
