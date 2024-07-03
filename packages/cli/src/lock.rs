use std::{
    collections::{hash_map::Entry, HashMap},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::plugin::CliPlugin;

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct DioxusLock {
    #[serde(skip)]
    pub path: PathBuf,
    pub plugins: HashMap<String, PluginLockData>,
}

impl DioxusLock {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            plugins: HashMap::new(),
        }
    }

    pub fn load() -> crate::error::Result<Self> {
        let crate_dir = dioxus_cli_config::crate_root()?;

        let crate_dir = crate_dir.as_path();

        let Some(dioxus_conf_file) = acquire_dioxus_lock(crate_dir) else {
            return Ok(Self::new(crate_dir.join(".dioxus/Dioxus.lock")));
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

    // TODO Check if the uses for this require the clones
    /// Save the lock file to disk, changing the plugin maps of the lock if they
    /// are passed in, otherwise saving what it currently has
    pub fn save(&mut self, plugins: Option<&Vec<CliPlugin>>) -> crate::error::Result<()> {
        let parent_path = self.path.parent().unwrap();

        if !parent_path.is_dir() {
            std::fs::create_dir_all(parent_path)?;
        }

        if let Some(plugins) = plugins {
            for plugin in plugins.iter() {
                self.plugins.insert(
                    plugin.metadata.name.clone(),
                    plugin.store.data().lock_data.clone(),
                );
            }
        }

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
        if !plugins.is_empty() {
            return Ok(());
        }

        for plugin in plugins.iter_mut() {
            self.initialize_plugin(plugin).await?;
        }

        self.save(Some(plugins))?;

        Ok(())
    }

    pub async fn initialize_plugin(&mut self, plugin: &mut CliPlugin) -> crate::error::Result<()> {
        let entry = self.plugins.entry(plugin.metadata.name.clone());
        if matches!(entry, Entry::Vacant(_)) {
            if let Err(err) = plugin.register().await? {
                tracing::warn!(
                    "Couldn't initialize plugin {}: {}",
                    plugin.metadata.name,
                    err
                );
            }
        }
        let state = entry.or_default();

        *state = plugin.store.data().lock_data.clone();
        Ok(())
    }

    pub async fn add_plugin(&mut self, plugin: &mut CliPlugin) -> crate::error::Result<()> {
        self.initialize_plugin(plugin).await?;

        self.save(None)?;

        Ok(())
    }
}

fn acquire_dioxus_lock(crate_dir: &Path) -> Option<PathBuf> {
    let dir = crate_dir.join(".dioxus");

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

#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PluginLockData {
    state: Option<toml::Value>,
}
