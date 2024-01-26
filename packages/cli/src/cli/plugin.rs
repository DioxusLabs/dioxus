use std::path::PathBuf;
use std::str::FromStr;

// use super::*;
use crate::lock::DioxusLock;
use crate::plugin::load_plugin;
use crate::plugin::PLUGINS_CONFIG;
use crate::DioxusConfig;
use crate::PluginConfigInfo;
use clap::Parser;
use clap::Subcommand;
use serde::Deserialize;

#[derive(Parser, Debug, Clone, PartialEq, Deserialize)]
pub enum PluginAdd {
    // Git {
    //   #[clap(short, long)]
    //   repo: String,
    //   #[clap(short, long)]
    //   branch: Option<String>,
    // }
    Add {
        // The path to the .wasm Plugin file
        #[clap(short, long)]
        path: PathBuf,
        // Optional priority value to change the order of how plugins are loaded
        #[clap(long)]
        priority: Option<usize>,
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
    pub async fn plugin(self, dx_config: &DioxusConfig) -> super::Result<()> {
        match self {
            // Plugin::Update { ignore_error } => todo!(),
            Plugin::List => {
                let plugins = &PLUGINS_CONFIG.lock().await.plugins.plugins;
                if plugins.is_empty() {
                    log::warn!(
                        "No plugins found! Run `dx config init` and then run `dx add --path /path/to/.wasm"
                    );
                    return Ok(());
                };

                for (name, data) in plugins.iter() {
                    log::info!("Found Plugin: {name} | Version {}", data.version);
                }
            }
            Plugin::Add(data) => match data {
                PluginAdd::Add { path, priority } => {
                    let mut dioxus_lock = DioxusLock::load()?;
                    let crate_dir = crate::crate_root()?;
                    let mut plugin =
                        load_plugin(&path, dx_config, &crate_dir, &dioxus_lock).await?;

                    // Add the plugin to the lock file
                    dioxus_lock.add_plugin(&mut plugin).await?;

                    // Redacted for now
                    // See issue: https://github.com/bytecodealliance/wit-bindgen/issues/817
                    // let res = plugin.get_default_config().await;
                    // let Ok(config) = res else {
                    //     log::warn!(
                    //         "Couldn't get default config from plugin: {} : {}",
                    //         plugin.metadata.name,
                    //         res.unwrap_err()
                    //     );
                    //     return Ok(());
                    // };

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
                        // config,
                        priority,
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
