use crate::build::TargetArgs;
use crate::{metadata::crate_root, CliSettings};

use super::*;

/// Dioxus config file controls
#[derive(Clone, Debug, Deserialize, Subcommand)]
#[clap(name = "config")]
pub enum Config {
    /// Init `Dioxus.toml` for project/folder.
    Init {
        /// Init project name
        name: String,

        /// Cover old config
        #[clap(long)]
        #[serde(default)]
        force: bool,

        /// Project default platform
        #[clap(long, default_value = "web")]
        platform: String,
    },
    /// Format print Dioxus config.
    FormatPrint {},
    /// Create a custom html file.
    CustomHtml {},

    /// Set CLI settings.
    #[command(subcommand)]
    Set(Setting),
}

#[derive(Debug, Clone, Copy, Deserialize, Subcommand)]
pub enum Setting {
    /// Set the value of the always-hot-reload setting.
    #[clap(action=ArgAction::Set)]
    AlwaysHotReload { value: bool },
    /// Set the value of the always-open-browser setting.
    #[clap(action=ArgAction::Set)]
    AlwaysOpenBrowser { value: bool },
    /// Set the value of the always-on-top desktop setting.
    #[clap(action=ArgAction::Set)]
    AlwaysOnTop { value: bool },
    /// Set the interval that file changes are polled on WSL for hot reloading.
    WSLFilePollInterval { value: u16 },
}

impl Display for Setting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlwaysHotReload { value: _ } => write!(f, "always_hot_reload"),
            Self::AlwaysOpenBrowser { value: _ } => write!(f, "always_open_browser"),
            Self::AlwaysOnTop { value: _ } => write!(f, "always_on_top"),
            Self::WSLFilePollInterval { value: _ } => write!(f, "wsl_file_poll_interval"),
        }
    }
}

impl Config {
    pub fn config(self) -> Result<()> {
        let crate_root = crate_root()?;
        match self {
            Config::Init {
                name,
                force,
                platform,
            } => {
                let conf_path = crate_root.join("Dioxus.toml");
                if conf_path.is_file() && !force {
                    tracing::warn!(
                        "config file `Dioxus.toml` already exist, use `--force` to overwrite it."
                    );
                    return Ok(());
                }
                let mut file = File::create(conf_path)?;
                let content = String::from(include_str!("../../assets/dioxus.toml"))
                    .replace("{{project-name}}", &name)
                    .replace("{{default-platform}}", &platform);
                file.write_all(content.as_bytes())?;
                tracing::info!("🚩 Init config file completed.");
            }
            Config::FormatPrint {} => {
                println!(
                    "{:#?}",
                    crate::dioxus_crate::DioxusCrate::new(&TargetArgs::default())?.dioxus_config
                );
            }
            Config::CustomHtml {} => {
                let html_path = crate_root.join("index.html");
                let mut file = File::create(html_path)?;
                let content = include_str!("../../assets/index.html");
                file.write_all(content.as_bytes())?;
                tracing::info!("🚩 Create custom html file done.");
            }
            // Handle CLI settings.
            Config::Set(setting) => {
                CliSettings::modify_settings(|settings| match setting {
                    Setting::AlwaysOnTop { value } => settings.always_on_top = Some(value.into()),
                    Setting::AlwaysHotReload { value } => {
                        settings.always_hot_reload = Some(value.into())
                    }
                    Setting::AlwaysOpenBrowser { value } => {
                        settings.always_open_browser = Some(value.into())
                    }
                    Setting::WSLFilePollInterval { value } => {
                        settings.wsl_file_poll_interval = Some(value.into())
                    }
                })?;
                tracing::info!("🚩 CLI setting `{setting}` has been set.");
            }
        }
        Ok(())
    }
}
