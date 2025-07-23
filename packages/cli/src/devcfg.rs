//! Configuration of the CLI at runtime to enable certain experimental features.

/// Should we force the entropy to be used on the main exe?
///
/// This is used to verify that binaries are copied with different names such that they don't collide
/// and should generally be only enabled on certain platforms that require it.
pub(crate) fn should_force_entropy() -> bool {
    std::env::var("DIOXUS_FORCE_ENTRY").is_ok()
}

/// Should the CLI not download any additional tools?
pub(crate) fn no_downloads() -> bool {
    std::env::var("NO_DOWNLOADS").is_ok()
}

/// Should we disable telemetry?
pub(crate) fn disable_telemetry() -> bool {
    match std::env::var("TELEMETRY") {
        Ok(val) => !val.eq_ignore_ascii_case("false") && !val.eq_ignore_ascii_case("0"),
        Err(_) => false,
    }
}

/// Should we test the installs?
#[allow(dead_code)] // -> used in tests only
pub(crate) fn test_installs() -> bool {
    std::env::var("TEST_INSTALLS").is_ok()
}
