use std::str::FromStr;

use super::*;
use crate::plugin::interface::exports::plugins::main::definitions::PluginInfo;
use crate::plugin::{convert::Convert, load_plugin};
use crate::{PluginConfig, PluginConfigInfo};
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
        let mut changed_config = false;
        match self {
            Plugin::Init { force, .. } => {
                let plugins = &mut crate_config.dioxus_config.plugins;
                if plugins.plugin.is_empty() {
                    log::warn!(
                        "No plugins found! Add a `[plugins.PLUGIN_NAME]` to your `Dioxus.toml!`"
                    );
                    return Ok(());
                }
                for (name, data) in plugins.plugin.iter() {
                    if !data.enabled {
                        log::info!("Plugin {} disabled, skipping..", name);
                        continue;
                    }

                    if data.initialized && !force {
                        log::info!("Plugin {} already initialized, skipping..", name);
                        continue;
                    }

                    let mut plugin = load_plugin(&data.path).await?;

                    if let Some(config) = plugins.config.get(name).cloned() {
                        let handle = plugin.insert_toml(config).await;
                        if plugin.apply_config(handle).await.is_err() {
                            log::warn!("Couldn't apply config from `Dioxus.toml` to {}!", name);
                            return Ok(()); // Skip maybe?
                        }
                    }
                }
                log::info!("🚩 Plugin init completed.");
            }
            Plugin::Refresh => {}
            // Plugin::Update { ignore_error } => todo!(),
            Plugin::List => {
                let plugins = &crate_config.dioxus_config.plugins.plugin;
                if plugins.is_empty() {
                    log::warn!("No plugins found! Run `dx config init` and Add a `[plugins.PLUGIN_NAME]` to your `Dioxus.toml`!");
                    return Ok(());
                };

                for (name, data) in plugins.iter() {
                    let enabled_icon = if data.enabled { "✔️" } else { "❌" };
                    log::info!(
                        "Found Plugin: {name} | Version {} | Enabled {enabled_icon}",
                        data.version
                    );
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
                        enabled: true,
                        initialized: true,
                    };

                    let plugins = &mut crate_config.dioxus_config.plugins;
                    plugins.set_plugin_info(name.clone(), new_config);
                    plugins.set_plugin_toml_config(&name, default_config);
                    changed_config = true;
                    dbg!(crate_config.dioxus_config.clone());
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
            let PluginConfig { plugin, config } = crate_config.dioxus_config.plugins;
            for (name, info) in plugin.into_iter() {
                let mut tab = toml_edit::table();
                let PluginConfigInfo {
                    version,
                    path,
                    enabled,
                    initialized,
                } = info;
                tab["version"] = toml_edit::value(version.to_string());
                tab["path"] = toml_edit::value(format!("{}", path.display()));
                tab["enabled"] = toml_edit::value(enabled);
                tab["initialized"] = toml_edit::value(initialized);
                diox_doc["plugins"]["plugin"][&name] = tab;
            }
            for (name, config) in config.into_iter() {
                diox_doc["plugins"]["config"][&name] = toml_edit::value(config.convert());
            }
            std::fs::write(toml_path, diox_doc.to_string())?;
        }

        Ok(())
    }
}
