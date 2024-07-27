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

    /// Set global cli settings.
    SetGlobal { setting: Setting, value: Value },
}

#[derive(Debug, Clone, Copy, Deserialize, clap::ValueEnum)]
pub enum Setting {
    /// Set the value of the always-hot-reload setting.
    AlwaysHotReload,
    /// Set the value of the always-open-browser setting.
    AlwaysOpenBrowser,
    /// Set the value of the always-on-top desktop setting.
    AlwaysOnTop,
}

impl Display for Setting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlwaysHotReload => write!(f, "always_hot_reload"),
            Self::AlwaysOpenBrowser => write!(f, "always_open_browser"),
            Self::AlwaysOnTop => write!(f, "always_on_top"),
        }
    }
}

// NOTE: Unsure of an alternative to get the desired behavior with clap, if it exists.
#[derive(Debug, Clone, Copy, Deserialize, clap::ValueEnum)]
pub enum Value {
    True,
    False,
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::True => write!(f, "true"),
            Self::False => write!(f, "false"),
        }
    }
}

impl From<Value> for bool {
    fn from(value: Value) -> Self {
        match value {
            Value::True => true,
            Value::False => false,
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
            // Handle configuration of global CLI settings.
            Config::SetGlobal { setting, value } => {
                CliSettings::modify_settings(|settings| match setting {
                    Setting::AlwaysHotReload => settings.always_hot_reload = Some(value.into()),
                    Setting::AlwaysOpenBrowser => settings.always_open_browser = Some(value.into()),
                    Setting::AlwaysOnTop => settings.always_on_top = Some(value.into()),
                })?;
                tracing::info!("🚩 CLI setting `{setting}` has been set to `{value}`")
            }
        }
        Ok(())
    }
}
