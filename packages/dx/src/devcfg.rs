//! Configuration of the CLI at runtime to enable certain experimental features.

/// Should we force the entropy to be used on the main exe?
///
/// This is used to verify that binaries are copied with different names such that they don't collide
/// and should generally be only enabled on certain platforms that require it.
pub(crate) fn should_force_entropy() -> bool {
    std::env::var("DIOXUS_FORCE_ENTRY").is_ok()
}

/// Should we test the installs?
#[allow(dead_code)] // -> used in tests only
pub(crate) fn test_installs() -> bool {
    std::env::var("TEST_INSTALLS").is_ok()
}
