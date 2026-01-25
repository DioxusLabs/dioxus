use super::*;
use crate::{CliSettings, TraceSrc, Workspace};

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
    },

    /// Format print Dioxus config.
    FormatPrint {},

    /// Create a custom html file.
    CustomHtml {},

    /// Set CLI settings.
    #[command(subcommand)]
    Set(Setting),

    /// Generate JSON schema for Dioxus.toml configuration.
    /// Useful for IDE autocomplete and validation.
    Schema {
        /// Output file path. If not provided, prints to stdout.
        #[clap(long, short)]
        out: Option<PathBuf>,
    },
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
    /// Disable the built-in telemetry for the CLI
    DisableTelemetry { value: BoolValue },
}

impl Display for Setting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlwaysHotReload { value: _ } => write!(f, "always-hot-reload"),
            Self::AlwaysOpenBrowser { value: _ } => write!(f, "always-open-browser"),
            Self::AlwaysOnTop { value: _ } => write!(f, "always-on-top"),
            Self::WSLFilePollInterval { value: _ } => write!(f, "wsl-file-poll-interval"),
            Self::DisableTelemetry { value: _ } => write!(f, "disable-telemetry"),
        }
    }
}

// Clap complains if we use a bool directly and I can't find much info about it.
// "Argument 'value` is positional and it must take a value but action is SetTrue"
#[derive(Debug, Clone, Copy, serde::Serialize, Deserialize, clap::ValueEnum)]
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
    pub(crate) async fn config(self) -> Result<StructuredOutput> {
        let crate_root = Workspace::crate_root_from_path()?;
        match self {
            Config::Init { name, force } => {
                let conf_path = crate_root.join("Dioxus.toml");
                if conf_path.is_file() && !force {
                    tracing::warn!(
                        "config file `Dioxus.toml` already exist, use `--force` to overwrite it."
                    );
                    return Ok(StructuredOutput::Success);
                }
                let mut file = File::create(conf_path)?;
                let content = String::from(include_str!("../../assets/dioxus.toml"))
                    .replace("{{project-name}}", &name);
                file.write_all(content.as_bytes())?;
                tracing::info!(dx_src = ?TraceSrc::Dev, "🚩 Init config file completed.");
            }
            Config::FormatPrint {} => {
                let workspace = Workspace::current().await?;
                tracing::info!("{:#?}", workspace.settings);
            }
            Config::CustomHtml {} => {
                let html_path = crate_root.join("index.html");
                let mut file = File::create(html_path)?;
                let content = include_str!("../../assets/web/dev.index.html");
                file.write_all(content.as_bytes())?;
                tracing::info!(dx_src = ?TraceSrc::Dev, "🚩 Create custom html file done.");
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
                    Setting::DisableTelemetry { value } => {
                        settings.disable_telemetry = Some(value.into());
                    }
                })?;
                tracing::info!(dx_src = ?TraceSrc::Dev, "🚩 CLI setting `{setting}` has been set.");
            }
            Config::Schema { out } => {
                let schema = crate::config::generate_manifest_schema();
                let json = serde_json::to_string_pretty(&schema)?;
                match out {
                    Some(path) => {
                        std::fs::write(&path, format!("{json}\n"))?;
                        tracing::info!(dx_src = ?TraceSrc::Dev, "Schema written to {}", path.display());
                    }
                    None => println!("{json}"),
                }
            }
        }

        Ok(StructuredOutput::Success)
    }
}
