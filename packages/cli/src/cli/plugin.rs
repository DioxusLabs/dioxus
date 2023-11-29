use super::*;

/// Build the Rust WASM app and all of its assets.
#[derive(Clone, Debug, Deserialize, Subcommand)]
#[clap(name = "plugin")]
pub enum Plugin {
    /// Init plugin system for current project
    Init {},
    /// Print all installed cli plugins.
    List {},
    /// Get plugin storage path.
    AppPath {},
    /// Upgrade plugin or library.
    Upgrade { name: String },
    /// Install a new plugin.
    Add {
        #[clap(long, default_value_t)]
        git: String,
    },
    /// Create a new plugin by plugin-develop template.
    Create {
        /// Open this flag will init some sumneko-lua vscode settings.
        #[clap(long, default_missing_value = "true")]
        vscode: bool,
    },
}

impl Plugin {
    pub async fn plugin(self) -> Result<()> {
        match self {
            Plugin::Init {} => {
                crate::plugin::PluginManager::init_plugin_dir();
            }
            Plugin::List {} => {
                let dioxus_config = crate::DioxusConfig::load(None)
                    .expect("Failed to load `Dioxus.toml`")
                    .unwrap_or_default();
                if crate::plugin::PluginManager::get_plugin_dir().is_none() {
                    log::warn!("Plugin system not available.");
                    log::warn!("Please execute `dioxus plugin init` command first.");
                } else {
                    crate::plugin::PluginManager::init(dioxus_config)
                        .expect("🚫 Plugin system initialization failed.");

                    for item in crate::plugin::PluginManager::plugin_list() {
                        println!("- {item}");
                    }
                }
            }
            Plugin::Upgrade { name } => {
                if name.to_lowercase() == "core" {
                    let upgrade = crate::plugin::PluginManager::upgrade_core_library(
                        crate::plugin::CORE_LIBRARY_VERSION,
                    );
                    if let Err(e) = upgrade {
                        log::error!("Plugin core library upgrade failed: {e}.");
                    } else {
                        println!("Plugin core library upgraded.");
                    }
                } else {
                    log::warn!("Plugin upgrade coming soon...");
                    log::warn!("Currently just support upgrade `core` library");
                }
            }
            Plugin::AppPath {} => {
                let plugin_dir = crate::plugin::PluginManager::get_plugin_dir();
                if let Some(plugin_dir) = plugin_dir {
                    if let Some(v) = plugin_dir.to_str() {
                        println!("{}", v);
                    } else {
                        log::error!("Plugin path get failed.");
                    }
                }
            }
            Plugin::Add { git } => {
                if !git.is_empty() {
                    if let Err(e) = crate::plugin::PluginManager::remote_install_plugin(git) {
                        log::error!("Plugin install failed: {e}");
                    } else {
                        println!("🔰 Plugin install done.");
                    }
                } else {
                    println!(
                        "Please use `dioxus plugin add --git {{GIT_URL}}` to install plugin.\n"
                    );
                    log::warn!("We are working for plugin index system, but for now, you need use git url to install plugin.\n");
                    println!("Maybe this link can help you to find some useful plugins: https://github.com/search?q=dioxus-plugin&type=repositories")
                }
            }
            Plugin::Create { vscode } => {
                if let Err(e) = crate::plugin::PluginManager::create_dev_plugin(vscode) {
                    log::error!("Plugin create failed: {e}")
                } else {
                    println!("🔰 Develop plugin create done.");
                }
            }
        }
        Ok(())
    }
}
