use std::str::FromStr;

use super::*;
use crate::lock::DioxusLock;
use crate::plugin::interface::exports::plugins::main::definitions::PluginInfo;
use crate::plugin::{convert::Convert, load_plugin};
use crate::PluginConfigInfo;
use clap::Parser;

#[derive(Parser, Debug, Clone, PartialEq, Deserialize)]
pub enum PluginAdd {
    // Git {
    //   #[clap(short, long)]
    //   repo: String,
    //   #[clap(short, long)]
    //   branch: Option<String>,
    // }
    Add {
        #[clap(short, long)]
        path: PathBuf,
    },
}

#[derive(Clone, Debug, Deserialize, Subcommand)]
#[clap(name = "plugin")]
pub enum Plugin {
    #[command(flatten)]
    Add(PluginAdd),

    // Go through each plugin and check for updates
    // Update {
    //   #[clap(long)]
    //   #[serde(default)]
    //   ignore_error: bool
    // },
    /// List all of the plugins installed
    List,
}

impl Plugin {
    pub async fn plugin(self, bin: Option<PathBuf>) -> Result<()> {
        let mut crate_config = crate::CrateConfig::new(bin)?;
        let mut changed_config = false;
        match self {
            // Plugin::Update { ignore_error } => todo!(),
            Plugin::List => {
                let plugins = &crate_config.dioxus_config.plugins.plugins;
                if plugins.is_empty() {
                    log::warn!("No plugins found! Run `dx config init` and Add a `[plugins.PLUGIN_NAME]` to your `Dioxus.toml`!");
                    return Ok(());
                };

                for (name, data) in plugins.iter() {
                    log::info!("Found Plugin: {name} | Version {}", data.version);
                }
            }
            Plugin::Add(data) => match data {
                PluginAdd::Add { path } => {
                    let mut plugin = load_plugin(&path).await?;

                    // Add the plugin to the lock file
                    let mut dioxus_lock = DioxusLock::load()?;
                    dioxus_lock.add_plugin(&mut plugin).await?;

                    let PluginInfo { name, version } = plugin.metadata().await?;

                    let Ok(default_config) = plugin.get_default_config().await else {
                        log::warn!("Couldn't get default config from plugin: {}", name);
                        return Ok(());
                    };

                    let Ok(version) = semver::Version::from_str(&version) else {
                        log::warn!(
                            "Couldn't parse version from plugin: {} >> {}",
                            name,
                            version
                        );
                        return Ok(());
                    };

                    let new_config = PluginConfigInfo {
                        version,
                        path,
                        config: default_config,
                    };

                    let plugins = &mut crate_config.dioxus_config.plugins;
                    plugins.set_plugin_info(name.clone(), new_config);
                    changed_config = true;
                    log::info!("✔️  Successfully added {name}");
                }
            },
        }

        if changed_config {
            let toml_path = crate_config.crate_dir.join("Dioxus.toml");
            let toml_string = std::fs::read_to_string(&toml_path)?;
            let mut diox_doc: toml_edit::Document = match toml_string.parse() {
                Ok(doc) => doc,
                Err(err) => {
                    log::warn!("Could not parse Dioxus toml! {}", err);
                    return Ok(());
                }
            };
            let val = toml::Value::try_from(&crate_config.dioxus_config.plugins)
                .expect("Invalid PluginInfo!");
            diox_doc["plugins"] = val.convert();
            std::fs::write(toml_path, diox_doc.to_string())?;
            log::info!("✔️  Successfully saved config");
        }

        Ok(())
    }
}
