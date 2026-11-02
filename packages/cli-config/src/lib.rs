//! This crate has been renamed to [`dx_env`] and now only re-exports it. Depend on `dx-env`
//! directly instead.

pub use dx_env::*;

#[deprecated(note = "Dioxus window settings moved into the renderer crates")]
pub const ALWAYS_ON_TOP_ENV: &str = "DIOXUS_ALWAYS_ON_TOP";

#[deprecated(note = "Dioxus window settings moved into the renderer crates")]
pub const APP_TITLE_ENV: &str = "DIOXUS_APP_TITLE";

/// Get the title of the application, usually set by the Dioxus.toml.
#[deprecated(note = "Dioxus window settings moved into the renderer crates")]
pub fn app_title() -> Option<String> {
    #[cfg(debug_assertions)]
    {
        std::env::var("DIOXUS_APP_TITLE").ok()
    }

    #[cfg(not(debug_assertions))]
    {
        option_env!("DIOXUS_APP_TITLE").map(ToString::to_string)
    }
}

/// Check if the application should forced to "float" on top of other windows.
#[deprecated(note = "Dioxus window settings moved into the renderer crates")]
pub fn always_on_top() -> Option<bool> {
    std::env::var("DIOXUS_ALWAYS_ON_TOP")
        .ok()
        .and_then(|s| s.parse().ok())
}
