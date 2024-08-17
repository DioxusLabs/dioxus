#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

mod config;
pub use config::*;

mod bundle;
pub use bundle::*;

mod serve;
pub use serve::*;

mod build_info;

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
    // todo: somehow support wasm?
    std::env::var("DIOXUS_CONFIG")
        .ok()
        .and_then(|config| serde_json::from_str(&config).ok())
        .ok_or_else(|| {
        //             let mut cli_version = crate::build_info::PKG_VERSION.to_string();

        //             if let Some(hash) = crate::build_info::GIT_COMMIT_HASH_SHORT {
        //                 let hash = &hash.trim_start_matches('g')[..4];
        //                 cli_version.push_str(&format!("-{hash}"));
        //             }

        //             tracing::warn!("Failed to parse the CLI config file. This is likely caused by a mismatch between the version of the CLI and the dioxus version.\nCLI version: {cli_version}\nDioxus version: {dioxus_version}\nSerialization error: {err}");
        //             Err(DioxusCLINotUsed)
        // #[doc(hidden)]
        // pub mod __private {
        //     use crate::DioxusConfig;

        //     pub(crate) const DIOXUS_CLI_VERSION: &str = "DIOXUS_CLI_VERSION";
        //     pub(crate) const CONFIG_ENV: &str = "DIOXUS_CONFIG";
        //     pub(crate) const CONFIG_BASE_PATH_ENV: &str = "DIOXUS_CONFIG_BASE_PATH";

        //     pub fn env_args() -> Vec<(&'static str, &'static str)> {
        //         vec![
        //             (CONFIG_ENV, "DIOXUS_CONFIG"),
        //             (CONFIG_BASE_PATH_ENV, "DIOXUS_CONFIG_BASE_PATH"),
        //             (DIOXUS_CLI_VERSION, "DIOXUS_CLI_VERSION"),
        //         ]
        //     }

        //     #[cfg(feature = "read-config")]
        //     /// The environment variable that stores the CLIs serve configuration.
        //     /// We use this to communicate between the CLI and the server for fullstack applications.
        //     pub const SERVE_ENV: &str = "DIOXUS_SERVE_CONFIG";
        // }

            tracing::warn!("A library is trying to access the crate's configuration, but the dioxus CLI was not used to build the application.");
            DioxusCLINotUsed
        })
});

/// Get the "base path" from the Dioxus.toml file
///
/// This is typically the offset of the asset from its domain
pub fn base_path() -> Option<&'static str> {
    todo!()
}
