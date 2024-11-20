use super::*;
use crate::TraceSrc;
use crate::{metadata::crate_root, CliSettings};

/// Dioxus config file controls
#[derive(Clone, Debug, Deserialize, Subcommand)]
pub(crate) enum Config {
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

    /// Print the location of the CLI log file.
    LogFile {},

    /// Set CLI settings.
    #[command(subcommand)]
    Set(Setting),
}

#[derive(Debug, Clone, Copy, Deserialize, Subcommand)]
pub(crate) enum Setting {
    /// Set the value of the always-hot-reload setting.
    AlwaysHotReload { value: BoolValue },
    /// Set the value of the always-open-browser setting.
    AlwaysOpenBrowser { value: BoolValue },
    /// Set the value of the always-on-top desktop setting.
    AlwaysOnTop { value: BoolValue },
    /// Set the interval that file changes are polled on WSL for hot reloading.
    WSLFilePollInterval { value: u16 },
}

impl Display for Setting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlwaysHotReload { value: _ } => write!(f, "always-hot-reload"),
            Self::AlwaysOpenBrowser { value: _ } => write!(f, "always-open-browser"),
            Self::AlwaysOnTop { value: _ } => write!(f, "always-on-top"),
            Self::WSLFilePollInterval { value: _ } => write!(f, "wsl-file-poll-interval"),
        }
    }
}

// Clap complains if we use a bool directly and I can't find much info about it.
// "Argument 'value` is positional and it must take a value but action is SetTrue"
#[derive(Debug, Clone, Copy, Deserialize, clap::ValueEnum)]
pub(crate) enum BoolValue {
    True,
    False,
}

impl From<BoolValue> for bool {
    fn from(value: BoolValue) -> Self {
        match value {
            BoolValue::True => true,
            BoolValue::False => false,
        }
    }
}

impl Config {
    pub(crate) fn config(self) -> Result<StructuredOutput> {
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
                    return Ok(StructuredOutput::Success);
                }
                let mut file = File::create(conf_path)?;
                let content = String::from(include_str!("../../assets/dioxus.toml"))
                    .replace("{{project-name}}", &name)
                    .replace("{{default-platform}}", &platform);
                file.write_all(content.as_bytes())?;
                tracing::info!(dx_src = ?TraceSrc::Dev, "🚩 Init config file completed.");
            }
            Config::FormatPrint {} => {
                tracing::info!(
                    "{:#?}",
                    crate::dioxus_crate::DioxusCrate::new(&TargetArgs::default())?.config
                );
            }
            Config::CustomHtml {} => {
                let html_path = crate_root.join("index.html");
                let mut file = File::create(html_path)?;
                let content = include_str!("../../assets/web/index.html");
                file.write_all(content.as_bytes())?;
                tracing::info!(dx_src = ?TraceSrc::Dev, "🚩 Create custom html file done.");
            }
            Config::LogFile {} => {
                let log_path = crate::logging::FileAppendLayer::log_path();
                tracing::info!(dx_src = ?TraceSrc::Dev, "Log file is located at {}", log_path.display());
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
                        settings.wsl_file_poll_interval = Some(value)
                    }
                })?;
                tracing::info!(dx_src = ?TraceSrc::Dev, "🚩 CLI setting `{setting}` has been set.");
            }
        }

        Ok(StructuredOutput::Success)
    }
}
