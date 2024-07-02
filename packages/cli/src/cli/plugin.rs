use std::path::PathBuf;

// use super::*;
use crate::lock::DioxusLock;
use crate::plugin::get_dependency_paths;
use crate::plugin::load_plugin;
use crate::plugin::PLUGINS_CONFIG;
use clap::Parser;
use clap::Subcommand;
use dioxus_cli_config::DioxusConfig;
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

    /// List all of the plugins installed
    List,
}

impl Plugin {
    pub async fn plugin(
        self,
        dx_config: &DioxusConfig,
        crate_dir: &PathBuf,
        dependency_paths: &[PathBuf],
    ) -> super::Result<()> {
        match self {
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
                    let mut plugin = load_plugin(
                        &path,
                        dx_config,
                        priority,
                        crate_dir,
                        &mut dioxus_lock,
                        dependency_paths,
                    )
                    .await?;

                    // Add the plugin to the lock file
                    dioxus_lock.add_plugin(&mut plugin).await?;

                    log::info!("✔️  Successfully added {}", plugin.metadata.name);
                }
            },
        }
        Ok(())
    }
}
