//! Configuration of the CLI at runtime to enable certain experimental features.

use std::path::Path;

/// Should we cache the dependency library?
///
/// When the `DIOXUS_CACHE_DEP_LIB` environment variable is set, we will cache the dependency library
/// built from the target's dependencies.
pub(crate) fn should_cache_dep_lib(lib: &Path) -> bool {
    std::env::var("DIOXUS_CACHE_DEP_LIB").is_ok() && lib.exists()
}

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

/// Should we test the installs?
#[allow(dead_code)] // -> used in tests only
pub(crate) fn test_installs() -> bool {
    std::env::var("TEST_INSTALLS").is_ok()
}
