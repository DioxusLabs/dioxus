#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

mod config;
pub use config::*;
mod bundle;
pub use bundle::*;
mod cargo;
pub use cargo::*;

#[doc(hidden)]
pub mod __private {
    use crate::CrateConfig;

    pub const CONFIG_ENV: &str = "DIOXUS_CONFIG";

    pub fn save_config(config: &CrateConfig) -> CrateConfigDropGuard {
        std::env::set_var(CONFIG_ENV, serde_json::to_string(config).unwrap());
        CrateConfigDropGuard
    }

    /// A guard that removes the config from the environment when dropped.
    pub struct CrateConfigDropGuard;

    impl Drop for CrateConfigDropGuard {
        fn drop(&mut self) {
            std::env::remove_var(CONFIG_ENV);
        }
    }
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
    Result<crate::config::CrateConfig, DioxusCLINotUsed>,
> = once_cell::sync::Lazy::new(|| {
    CURRENT_CONFIG_JSON
        .and_then(|config| serde_json::from_str(config).ok())
        .ok_or_else(|| {
            tracing::warn!("A library is trying to access the crate's configuration, but the dioxus CLI was not used to build the application.");
            DioxusCLINotUsed
    })
});

/// Get just the base path from the config without pulling in serde_json
pub const fn current_config_base_path() -> Option<&'static str> {
    // Find "base_path": "path/to/base"
    match CURRENT_CONFIG_JSON {
        Some(json) => {
            let mut index = 0;
            let mut search_index = 0;
            let search_for = r#""base_path":""#;
            while index < json.len() {
                let char = json.as_bytes()[index];
                if char == b' ' {
                    index += 1;
                    continue;
                }
                if char == search_for.as_bytes()[search_index] {
                    search_index += 1;
                } else {
                    search_index = 0;
                }
                if search_index == search_for.len() {
                    // Find the end of the string
                    let mut end_index = index + 1;
                    while end_index < json.len() {
                        let char = json.as_bytes()[end_index];
                        if char == b'"' {
                            break;
                        }
                        end_index += 1;
                    }
                    let (_, after_start) = json.as_bytes().split_at(index + 1);
                    let (before_end, _) = after_start.split_at(end_index - index - 1);
                    // SAFETY: We are slicing into a valid UTF-8 string
                    return Some(unsafe { std::str::from_utf8_unchecked(before_end) });
                }
                index += 1
            }
            None
        }
        None => None,
    }
}

#[cfg(feature = "read-config")]
/// The current crate's configuration.
pub const CURRENT_CONFIG_JSON: Option<&str> = std::option_env!("DIOXUS_CONFIG");
