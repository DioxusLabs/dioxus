use crate::{Result, TraceSrc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Arc};
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
    #[serde(default = "default_wsl_file_poll_interval")]
    pub(crate) wsl_file_poll_interval: Option<u16>,
    /// Use tooling from path rather than downloading them.
    pub(crate) no_downloads: Option<bool>,
}

impl CliSettings {
    /// Load the settings from the local, global, or default config in that order
    pub(crate) fn load() -> Arc<Self> {
        static SETTINGS: Lazy<Arc<CliSettings>> =
            Lazy::new(|| Arc::new(CliSettings::from_global().unwrap_or_default()));
        SETTINGS.clone()
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

    /// Save the current structure to the global settings toml.
    /// This does not save to project-level settings.
    pub(crate) fn save(&self) -> Result<()> {
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

        Ok(())
    }

    /// Get the path to the settings toml file.
    pub(crate) fn get_settings_path() -> Option<PathBuf> {
        let Some(path) = dirs::data_local_dir() else {
            warn!("failed to get local data directory, some config keys may be missing");
            return None;
        };

        Some(path.join(GLOBAL_SETTINGS_FILE_NAME))
    }

    /// Modify the settings toml file - doesn't change the settings for this session
    pub(crate) fn modify_settings(with: impl FnOnce(&mut CliSettings)) -> Result<()> {
        let mut _settings = CliSettings::load();
        let settings: &mut CliSettings = Arc::make_mut(&mut _settings);
        with(settings);
        settings.save()?;

        Ok(())
    }

    /// Check if we should prefer to use the no-downloads feature
    pub(crate) fn prefer_no_downloads() -> bool {
        if cfg!(feature = "no-downloads") {
            return true;
        }

        if std::env::var("NO_DOWNLOADS").is_ok() {
            return true;
        }

        CliSettings::load().no_downloads.unwrap_or_default()
    }
}

fn default_wsl_file_poll_interval() -> Option<u16> {
    Some(2)
}
