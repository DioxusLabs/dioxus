use crate::{Result, TraceSrc};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tracing::{error, trace, warn};

const GLOBAL_SETTINGS_FILE_NAME: &str = "dioxus/settings.toml";

/// Describes cli settings from project or global level.
/// The order of priority goes:
/// 1. CLI Flags/Arguments
/// 2. Project-level Settings
/// 3. Global-level settings.
///
/// This allows users to control the cli settings with ease.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct CliSettings {
    /// Describes whether hot reload should always be on.
    pub(crate) always_hot_reload: Option<bool>,
    /// Describes whether the CLI should always open the browser for Web targets.
    pub(crate) always_open_browser: Option<bool>,
    /// Describes whether desktop apps in development will be pinned always-on-top.
    pub(crate) always_on_top: Option<bool>,
    /// Describes the interval in seconds that the CLI should poll for file changes on WSL.
    pub(crate) wsl_file_poll_interval: Option<u16>,
}

impl CliSettings {
    pub(crate) fn should_hot_reload(&self) -> bool {
        self.always_hot_reload.unwrap_or(true)
    }

    pub(crate) fn should_open_browser(&self) -> bool {
        self.always_open_browser.unwrap_or(true)
    }

    pub(crate) fn get_always_on_top(&self) -> bool {
        self.always_on_top.unwrap_or(true)
    }

    pub(crate) fn get_wsl_file_poll_interval(&self) -> u16 {
        self.wsl_file_poll_interval.unwrap_or(2)
    }

    /// Load the settings from the local, global, or default config in that order
    pub(crate) fn load(settings_override: Option<Self>) -> Self {
        let mut settings = Self::from_global().unwrap_or_default();

        // Handle overriding settings from command args.
        if let Some(settings_override) = settings_override {
            if settings_override.always_hot_reload.is_some() {
                settings.always_hot_reload = settings_override.always_hot_reload
            }

            if settings_override.always_open_browser.is_some() {
                settings.always_open_browser = settings_override.always_open_browser;
            }

            if settings_override.always_on_top.is_some() {
                settings.always_on_top = settings_override.always_on_top;
            }

            if settings_override.wsl_file_poll_interval.is_some() {
                settings.wsl_file_poll_interval = settings_override.wsl_file_poll_interval;
            }
        }

        settings
    }

    /// Get the current settings structure from global.
    pub(crate) fn from_global() -> Option<Self> {
        let Some(path) = dirs::data_local_dir() else {
            warn!("failed to get local data directory, some config keys may be missing");
            return None;
        };

        let path = path.join(GLOBAL_SETTINGS_FILE_NAME);
        let Some(data) = fs::read_to_string(path).ok() else {
            // We use a debug here because we expect the file to not exist.
            trace!("failed to read `{}` config file", GLOBAL_SETTINGS_FILE_NAME);
            return None;
        };

        let data = toml::from_str::<CliSettings>(&data).ok();
        if data.is_none() {
            warn!(
                "failed to parse `{}` config file",
                GLOBAL_SETTINGS_FILE_NAME
            );
        }

        data
    }

    /// Modify the settings toml file
    pub(crate) fn modify_settings(with: impl FnOnce(&mut CliSettings)) -> Result<()> {
        let mut settings = Self::load(None);
        with(&mut settings);
        settings.save()?;

        Ok(())
    }

    /// Save the current structure to the global settings toml.
    /// This does not save to project-level settings.
    fn save(self) -> Result<Self> {
        let path = Self::get_settings_path().ok_or_else(|| {
            error!(dx_src = ?TraceSrc::Dev, "failed to get settings path");
            anyhow::anyhow!("failed to get settings path")
        })?;

        let data = toml::to_string_pretty(&self).map_err(|e| {
            error!(dx_src = ?TraceSrc::Dev, ?self, "failed to parse config into toml");
            anyhow::anyhow!("failed to parse config into toml: {e}")
        })?;

        // Create the directory structure if it doesn't exist.
        let parent_path = path.parent().unwrap();
        if let Err(e) = fs::create_dir_all(parent_path) {
            error!(
                dx_src = ?TraceSrc::Dev,
                ?data,
                ?path,
                "failed to create directories for settings file"
            );
            return Err(
                anyhow::anyhow!("failed to create directories for settings file: {e}").into(),
            );
        }

        // Write the data.
        let result = fs::write(&path, data.clone());
        if let Err(e) = result {
            error!(?data, ?path, "failed to save global cli settings");
            return Err(anyhow::anyhow!("failed to save global cli settings: {e}").into());
        }

        Ok(self)
    }

    /// Get the path to the settings toml file.
    fn get_settings_path() -> Option<PathBuf> {
        let Some(path) = dirs::data_local_dir() else {
            warn!("failed to get local data directory, some config keys may be missing");
            return None;
        };

        Some(path.join(GLOBAL_SETTINGS_FILE_NAME))
    }
}
