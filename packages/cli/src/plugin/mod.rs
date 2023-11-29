use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::Mutex,
};

use serde_json::json;

use crate::{
    tools::{app_path, clone_repo},
    CrateConfig,
};

use self::types::PluginConfig;

mod interface;
mod types;

pub struct PluginManager;

impl PluginManager {
    pub fn init(config: toml::Value) -> anyhow::Result<()> {
        let config = PluginConfig::from_toml_value(config);

        if !config.available {
            return Ok(());
        }

        Ok(())
    }

    pub fn on_build_start(crate_config: &CrateConfig, platform: &str) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn on_build_finish(crate_config: &CrateConfig, platform: &str) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn on_serve_start(crate_config: &CrateConfig) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn on_serve_rebuild(timestamp: i64, files: Vec<PathBuf>) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn on_serve_shutdown(crate_config: &CrateConfig) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn init_plugin_dir() -> PathBuf {
        let app_path = app_path();
        let plugin_path = app_path.join("plugins");
        if !plugin_path.is_dir() {
            log::info!("📖 Start to init plugin library ...");
            let url = "https://github.com/DioxusLabs/cli-plugin-library";
            if let Err(err) = clone_repo(&plugin_path, url) {
                log::error!("Failed to init plugin dir, error caused by {}. ", err);
            }
        }
        plugin_path
    }

    pub fn plugin_list() -> Vec<String> {
        let mut res = vec![];

        res
    }
}
