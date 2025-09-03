// The trybuild output has slightly different error message output for
// different combinations of features. Since tests are run with `test-all-features`
// multiple combinations of features are tested. This ensures this file is only
// run when **only** the browser feature is enabled.
#![cfg(all(
    rustc_nightly,
    feature = "browser",
    not(any(
        feature = "postcard",
        feature = "multipart",
        feature = "serde-lite",
        feature = "cbor",
        feature = "msgpack"
    ))
))]

#[test]
fn aliased_results() {
    let t = trybuild::TestCases::new();
    t.pass("tests/valid/*.rs");
    t.compile_fail("tests/invalid/*.rs")
}
