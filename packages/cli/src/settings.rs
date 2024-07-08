use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Error, ErrorKind},
    path::PathBuf,
};
use tracing::{debug, error, warn};

use crate::CrateConfigError;

const GLOBAL_SETTINGS_FILE_NAME: &str = "dioxus/settings.toml";

/// Describes cli settings from project or global level.
/// The order of priority goes:
/// 1. CLI Flags/Arguments
/// 2. Project-level Settings
/// 3. Global-level settings.
///
/// This allows users to control the cli settings with ease.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CliSettings {
    /// Describes whether hot reload should always be on.
    pub always_hot_reload: Option<bool>,
    /// Describes whether the CLI should always open the browser for Web targets.
    pub always_open_browser: Option<bool>,
}

impl CliSettings {
    /// Load the settings from the local, global, or default config in that order
    pub fn load() -> Self {
        Self::from_global().unwrap_or_default()
    }

    /// Get the current settings structure from global.
    pub fn from_global() -> Option<Self> {
        let Some(path) = dirs::data_local_dir() else {
            warn!("failed to get local data directory, some config keys may be missing");
            return None;
        };

        let path = path.join(GLOBAL_SETTINGS_FILE_NAME);
        let Some(data) = fs::read_to_string(path).ok() else {
            // We use a debug here because we expect the file to not exist.
            debug!("failed to read `{}` config file", GLOBAL_SETTINGS_FILE_NAME);
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
    pub fn save(self) -> Result<Self, CrateConfigError> {
        let path = Self::get_settings_path().ok_or_else(|| {
            error!("failed to get settings path");
            CrateConfigError::Io(Error::new(
                ErrorKind::NotFound,
                "failed to get settings path",
            ))
        })?;

        let data = toml::to_string_pretty(&self).map_err(|e| {
            error!(?self, "failed to parse config into toml");
            CrateConfigError::Io(Error::new(ErrorKind::Other, e.to_string()))
        })?;

        let result = fs::write(path.clone(), data.clone());
        if let Err(e) = result {
            error!(?data, ?path, "failed to save global cli settings");
            return Err(CrateConfigError::Io(e));
        }

        Ok(self)
    }

    /// Get the path to the settings toml file.
    pub fn get_settings_path() -> Option<PathBuf> {
        let Some(path) = dirs::data_local_dir() else {
            warn!("failed to get local data directory, some config keys may be missing");
            return None;
        };

        Some(path.join(GLOBAL_SETTINGS_FILE_NAME))
    }

    /// Modify the settings toml file
    pub fn modify_settings(with: impl FnOnce(&mut CliSettings)) -> Result<(), CrateConfigError> {
        let mut settings = Self::load();
        with(&mut settings);
        settings.save()?;

        Ok(())
    }
}
