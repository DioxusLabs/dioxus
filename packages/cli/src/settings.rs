use crate::{Result, TraceSrc};
use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use std::{fs, path::PathBuf, sync::Arc};
use tracing::{error, trace, warn};

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
    /// Ignore updates for this version
    pub(crate) ignore_version_update: Option<String>,
    /// Disable telemetry
    pub(crate) disable_telemetry: Option<bool>,
}

impl CliSettings {
    /// Load the settings from the local, global, or default config in that order
    pub(crate) fn load() -> Arc<Self> {
        static SETTINGS: LazyLock<Arc<CliSettings>> =
            LazyLock::new(|| Arc::new(CliSettings::global_or_default()));
        SETTINGS.clone()
    }

    pub fn global_or_default() -> Self {
        CliSettings::from_global().unwrap_or_default()
    }

    /// Get the path to the settings toml file.
    pub(crate) fn get_settings_path() -> PathBuf {
        crate::Workspace::global_settings_file()
    }

    /// Get the current settings structure from global.
    pub(crate) fn from_global() -> Option<Self> {
        let settings = crate::Workspace::global_settings_file();

        if !settings.exists() {
            trace!("global settings file does not exist, returning None");
            return None;
        }

        let Some(data) = fs::read_to_string(&settings).ok() else {
            warn!("failed to read global settings file");
            return None;
        };

        let Some(data) = toml::from_str::<CliSettings>(&data).ok() else {
            warn!("failed to parse global settings file");
            return None;
        };

        Some(data)
    }

    /// Save the current structure to the global settings toml.
    /// This does not save to project-level settings.
    pub(crate) fn save(&self) -> Result<()> {
        let path = Self::get_settings_path();

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
            bail!("failed to create directories for settings file: {e}");
        }

        // Write the data.
        let result = fs::write(&path, data.clone());
        if let Err(e) = result {
            error!(?data, ?path, "failed to save global cli settings");
            bail!("failed to save global cli settings: {e}");
        }

        Ok(())
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
        if cfg!(feature = "no-downloads") && !cfg!(debug_assertions) {
            return true;
        }

        if std::env::var("NO_DOWNLOADS").is_ok() {
            return true;
        }

        CliSettings::load().no_downloads.unwrap_or_default()
    }

    /// Check if telemetry is disabled
    pub(crate) fn telemetry_disabled() -> bool {
        use std::env::var;

        static TELEMETRY_DISABLED: LazyLock<bool> = LazyLock::new(|| {
            if cfg!(feature = "disable-telemetry") {
                return true;
            }

            if matches!(var("DX_TELEMETRY_ENABLED"), Ok(val) if val.eq_ignore_ascii_case("false") || val == "0")
            {
                return true;
            }

            if matches!(var("TELEMETRY"), Ok(val) if val.eq_ignore_ascii_case("false") || val == "0")
            {
                return true;
            }

            CliSettings::load().disable_telemetry.unwrap_or_default()
        });

        *TELEMETRY_DISABLED
    }

    pub(crate) fn is_ci() -> bool {
        static CI: LazyLock<bool> = LazyLock::new(|| {
            if matches!(std::env::var("CI"), Ok(val) if val.eq_ignore_ascii_case("true") || val == "1")
            {
                return true;
            }

            if matches!(std::env::var("DX_CI"), Ok(val) if val.eq_ignore_ascii_case("true") || val == "1")
            {
                return true;
            }

            false
        });

        *CI
    }
}

fn default_wsl_file_poll_interval() -> Option<u16> {
    Some(2)
}
