use super::*;

/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, Deserialize, Subcommand)]
#[clap(name = "plugin")]
pub enum Plugin {
    /// Return all dioxus-cli support tools.
    List {},
    /// Get default app install path.
    AppPath {},
    /// Install a new tool.
    Add { name: String },
}

impl Plugin {
    pub async fn plugin(self) -> Result<()> {
        match self {
            Plugin::List {} => {
                for item in crate::plugin::PluginManager::plugin_list() {
                    println!("- {item}");
                }
            }
            Plugin::AppPath {} => {
                if let Ok(plugin_dir) = crate::plugin::PluginManager::init_plugin_dir() {
                    if let Some(v) = plugin_dir.to_str() {
                        println!("{}", v);
                    } else {
                        log::error!("Plugin path get failed.");
                    }
                }
            }
            Plugin::Add { name: _ } => {
                log::info!("You can use `dioxus plugin app-path` to get Installation position");
            }
        }
        Ok(())
    }
}
