use crate::plugin::load_plugin;

use super::*;

#[derive(Clone, Debug, Deserialize, Subcommand)]
#[clap(name = "plugin")]
pub enum Plugin {
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
        let crate_config = crate::CrateConfig::new(bin)?;
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
        }
        Ok(())
    }
}
