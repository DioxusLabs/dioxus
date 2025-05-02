//! Configuration of the CLI at runtime to enable certain experimental features.

use std::path::Path;

/// Should we cache the dependency library?
///
/// When the `DIOXUS_CACHE_DEP_LIB` environment variable is set, we will cache the dependency library
/// built from the target's dependencies.
pub fn should_cache_dep_lib(lib: &Path) -> bool {
    std::env::var("DIOXUS_CACHE_DEP_LIB").is_ok() && lib.exists()
}
