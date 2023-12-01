use std::str::FromStr;

use crate::PluginConfig;
use crate::plugin::load_plugin;
use crate::plugin::interface::exports::plugins::main::definitions::PluginInfo;
use clap::Parser;
use super::*;

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
    }
}

#[derive(Clone, Debug, Deserialize, Subcommand)]
#[clap(name = "plugin")]
pub enum Plugin {
  #[command(flatten)]
    Add(PluginAdd),
    Init {
        #[clap(long)]
        #[serde(default)]
        force: bool,
    },
    // Go through each plugin and check for updates
    // Update {
    //   #[clap(long)]
    //   #[serde(default)]
    //   ignore_error: bool
    // },
    /// Checks the config for any more plugins that have been added,
    /// if there is register them and add them to the `Dioxus.toml`
    Refresh,

    /// List all of the plugins installed
    List,
}

impl Plugin {
    pub async fn plugin(self, bin: Option<PathBuf>) -> Result<()> {
        let mut crate_config = crate::CrateConfig::new(bin)?;
        match self {
            Plugin::Init { force, .. } => {
                let Some(plugins) = crate_config.dioxus_config.plugins else {
                    log::warn!(
                        "No plugins found! Add a `[plugins.PLUGIN_NAME]` to your `Dioxus.toml!`"
                    );
                    return Ok(());
                };
                for (name, data) in plugins.iter() {
                    dbg!((name, data));
                    if !data.enabled {
                        log::info!("Plugin {} disabled, skipping..", name);
                        continue;
                    }

                    if data.initialized && !force {
                        log::info!("Plugin {} already initialized, skipping..", name);
                        continue;
                    }

                    let mut plugin = load_plugin(&data.path).await?;
                    if plugin.register().await.is_err() {
                        log::warn!("Plugin {name} failed to register!");
                        continue;
                    } else {
                        log::info!("Plugin {name} successfully initialized");
                    }
                }
                log::info!("🚩 Plugin init completed.");
            }
            Plugin::Refresh => {}
            // Plugin::Update { ignore_error } => todo!(),
            Plugin::List => {
                let Some(plugins) = crate_config.dioxus_config.plugins else {
                    log::warn!("No plugins found! Run `dx config init` and Add a `[plugins.PLUGIN_NAME]` to your `Dioxus.toml`!");
                    return Ok(());
                };
                for (name, data) in plugins.into_iter() {
                    let enabled_icon = if data.enabled { "✔️" } else { "❌" };
                    log::info!("Found Plugin: {name} | Version {} | Enabled {enabled_icon} | Config = {:#?}", data.version, data.config)
                }
            }
            Plugin::Add(data) => match data {
                PluginAdd::Add { path } => {
                  let mut plugin = load_plugin(&path).await?;
                  
                  // Todo handle errors
                  let Ok(PluginInfo { name, version }) = plugin.register().await? else {
                    log::warn!("Couldn't load plugin from path: {}", path.display());
                    return Ok(());
                  };

                  let Ok(default_config) = plugin.get_default_config().await else {
                    log::warn!("Couldn't get default plugin from plugin: {}", name);
                    return Ok(())
                  };

                  let Ok(version) = semver::Version::from_str(&version) else {
                    log::warn!("Couldn't parse version from plugin: {} >> {}", name, version);
                    return Ok(())
                  };

                  let new_config = PluginConfig {
                    version,
                    path,
                    enabled: true,
                    initialized: true,
                    config: Some(default_config),
                };
                  
                  crate_config.dioxus_config.set_plugin_info(name, new_config);
                  dbg!(crate_config.dioxus_config);
                },
            },
        }
        Ok(())
    }
}
