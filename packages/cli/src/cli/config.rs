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

#[derive(Debug, Clone, Deserialize, clap::ValueEnum)]
pub enum Setting {
    /// Set the value of the always-hot-reload setting.
    AlwaysHotReload,
    /// Set the value of the always-open-browser setting.
    AlwaysOpenBrowser,
}

// NOTE: Unsure of an alternative to get the desired behavior with clap, if it exists.
#[derive(Debug, Clone, Deserialize, clap::ValueEnum)]
pub enum Value {
    True,
    False,
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
            Config::SetGlobal { setting, value } => {
                CliSettings::modify_settings(|settings| match setting {
                    Setting::AlwaysHotReload => settings.always_hot_reload = Some(value.into()),
                    Setting::AlwaysOpenBrowser => settings.always_open_browser = Some(value.into()),
                })?;
            }
        }
        Ok(())
    }
}
