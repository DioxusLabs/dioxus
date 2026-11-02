//! # Dioxus CLI configuration
//!
//! Dioxus-specific settings that `dx` passes to Dioxus apps, such as the window title. The
//! renderers read these when building their default window.
//!
//! The generic environment variable contract between `dx` and the app it launches lives in
//! [`dx_env`], which this crate re-exports.

pub use dx_env::*;

pub const ALWAYS_ON_TOP_ENV: &str = "DIOXUS_ALWAYS_ON_TOP";
pub const APP_TITLE_ENV: &str = "DIOXUS_APP_TITLE";

/// Get the title of the application, usually set by the Dioxus.toml.
///
/// This is used to set the title of the desktop window if the app itself doesn't set it.
///
/// `dx` sets this at runtime during development and at compile time for release builds, so bundled
/// apps keep the title when run without `dx`.
pub fn app_title() -> Option<String> {
    #[cfg(debug_assertions)]
    {
        std::env::var(APP_TITLE_ENV).ok()
    }

    #[cfg(not(debug_assertions))]
    {
        option_env!("DIOXUS_APP_TITLE").map(ToString::to_string)
    }
}

/// Check if the application should forced to "float" on top of other windows.
///
/// The CLI sets this based on the `--always-on-top` flag and the settings system.
pub fn always_on_top() -> Option<bool> {
    std::env::var(ALWAYS_ON_TOP_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
}
