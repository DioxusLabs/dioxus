#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

mod config;
pub use config::*;

mod bundle;
pub use bundle::*;

mod serve;
pub use serve::*;

#[doc(hidden)]
pub mod __private {
    use crate::DioxusConfig;

    pub(crate) const CONFIG_ENV: &str = "DIOXUS_CONFIG";
    pub(crate) const CONFIG_BASE_PATH_ENV: &str = "DIOXUS_CONFIG_BASE_PATH";

    pub fn save_config(config: &DioxusConfig) -> CrateConfigDropGuard {
        std::env::set_var(CONFIG_ENV, serde_json::to_string(config).unwrap());
        std::env::set_var(
            CONFIG_BASE_PATH_ENV,
            config.web.app.base_path.clone().unwrap_or_default(),
        );
        CrateConfigDropGuard
    }

    /// A guard that removes the config from the environment when dropped.
    pub struct CrateConfigDropGuard;

    impl Drop for CrateConfigDropGuard {
        fn drop(&mut self) {
            std::env::remove_var(CONFIG_ENV);
            std::env::remove_var(CONFIG_BASE_PATH_ENV);
        }
    }

    #[cfg(feature = "read-config")]
    /// The environment variable that stores the CLIs serve configuration.
    /// We use this to communicate between the CLI and the server for fullstack applications.
    pub const SERVE_ENV: &str = "DIOXUS_SERVE_CONFIG";
}

/// An error that occurs when the dioxus CLI was not used to build the application.
#[derive(Debug)]
pub struct DioxusCLINotUsed;

impl std::fmt::Display for DioxusCLINotUsed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("dioxus CLI was not used to build the application")
    }
}

impl std::error::Error for DioxusCLINotUsed {}

#[cfg(feature = "read-config")]
/// The current crate's configuration.
pub static CURRENT_CONFIG: once_cell::sync::Lazy<
    Result<crate::config::DioxusConfig, DioxusCLINotUsed>,
> = once_cell::sync::Lazy::new(|| {
    CURRENT_CONFIG_JSON
        .and_then(|config| serde_json::from_str(config).ok())
        .ok_or_else(|| {
            tracing::warn!("A library is trying to access the crate's configuration, but the dioxus CLI was not used to build the application.");
            DioxusCLINotUsed
    })
});

#[cfg(feature = "read-config")]
/// The current crate's configuration.
pub const CURRENT_CONFIG_JSON: Option<&str> = std::option_env!("DIOXUS_CONFIG");

#[cfg(feature = "read-config")]
/// The current crate's configuration.
pub const BASE_PATH: Option<&str> = std::option_env!("DIOXUS_CONFIG_BASE_PATH");
