use std::str::FromStr;

use super::*;
use crate::lock::DioxusLock;
use crate::plugin::load_plugin;
use crate::plugin::PLUGINS_CONFIG;
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
    pub async fn plugin(self) -> Result<()> {
        match self {
            // Plugin::Update { ignore_error } => todo!(),
            Plugin::List => {
                let plugins = &PLUGINS_CONFIG.lock().await.plugins.plugins;
                if plugins.is_empty() {
                    log::warn!(
                        "No plugins found! Run `dx config init` and then run `dx add --path WASM"
                    );
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

                    let Ok(default_config) = plugin.get_default_config().await else {
                        log::warn!(
                            "Couldn't get default config from plugin: {}",
                            plugin.metadata.name
                        );
                        return Ok(());
                    };

                    let Ok(version) = semver::Version::from_str(&plugin.metadata.version) else {
                        log::warn!(
                            "Couldn't parse version from plugin: {} >> {}",
                            plugin.metadata.name,
                            plugin.metadata.version
                        );
                        return Ok(());
                    };

                    let new_config = PluginConfigInfo {
                        version,
                        path,
                        config: default_config,
                    };

                    let plugins = &mut PLUGINS_CONFIG.lock().await.plugins;
                    plugins.set_plugin_info(plugin.metadata.name.clone(), new_config);
                    log::info!("✔️  Successfully added {}", plugin.metadata.name);
                }
            },
        }
        Ok(())
    }
}
