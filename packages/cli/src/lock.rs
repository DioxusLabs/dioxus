use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::plugin::{interface::plugins::main::imports::PluginInfo, CliPlugin};

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub(crate) struct DioxusLock {
    #[serde(skip)]
    path: PathBuf,
    plugins: HashMap<String, PluginState>,
}

impl DioxusLock {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            plugins: HashMap::new(),
        }
    }

    pub fn load() -> crate::error::Result<Self> {
        let crate_dir = crate::cargo::crate_root()?;

        let crate_dir = crate_dir.as_path();

        let Some(dioxus_conf_file) = acquire_dioxus_lock(crate_dir) else {
            return Ok(Self::new(crate_dir.join("Dioxus.lock")));
        };

        let dioxus_conf_file = dioxus_conf_file.as_path();
        let mut myself = toml::from_str::<Self>(&std::fs::read_to_string(dioxus_conf_file)?)
            .map_err(|err| {
                let error_location = dioxus_conf_file
                    .strip_prefix(crate_dir)
                    .unwrap_or(dioxus_conf_file)
                    .display();
                crate::Error::Unique(format!("{error_location} {err}"))
            })?;

        myself.path = dioxus_conf_file.to_path_buf();

        Ok(myself)
    }

    pub fn save(&self) -> crate::error::Result<()> {
        std::fs::create_dir_all(self.path.parent().unwrap())?;
        std::fs::write(
            &self.path,
            toml::to_string_pretty(self).map_err(|err| anyhow::anyhow!(err))?,
        )
        .map_err(|err| {
            let error_location = self.path.display();
            crate::Error::Unique(format!("{error_location} {err}"))
        })
    }

    pub async fn initialize_new_plugins(
        &mut self,
        plugins: &mut Vec<CliPlugin>,
    ) -> crate::error::Result<()> {
        let mut new_plugins = HashMap::new();
        for plugin in &mut *plugins {
            let PluginInfo { name, .. } = plugin.metadata().await?;
            let state = self.plugins.entry(name.clone()).or_default();
            if !state.initialized {
                match plugin.register().await? {
                    Ok(()) => {
                        state.initialized = true;
                    }
                    Err(_) => {
                        log::warn!("Couldn't initialize plugin: {}", name);
                    }
                }
            }
            new_plugins.insert(name, state.clone());
        }

        self.plugins = new_plugins;

        if !plugins.is_empty() {
            self.save()?;
        }

        Ok(())
    }

    pub async fn add_plugin(&mut self, plugin: &mut CliPlugin) -> crate::error::Result<()> {
        let PluginInfo { name, .. } = plugin.metadata().await?;

        let state = self.plugins.entry(name.clone()).or_default();
        if !state.initialized {
            match plugin.register().await? {
                Ok(()) => {
                    state.initialized = true;
                }
                Err(_) => {
                    log::warn!("Couldn't initialize plugin: {}", name);
                }
            }
        }

        self.save()?;

        Ok(())
    }
}

fn acquire_dioxus_lock(dir: &Path) -> Option<PathBuf> {
    // prefer uppercase
    let uppercase_conf = dir.join("Dioxus.lock");
    if uppercase_conf.is_file() {
        return Some(uppercase_conf);
    }

    // lowercase is fine too
    let lowercase_conf = dir.join("dioxus.lock");
    if lowercase_conf.is_file() {
        return Some(lowercase_conf);
    }

    None
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct PluginState {
    initialized: bool,
}
