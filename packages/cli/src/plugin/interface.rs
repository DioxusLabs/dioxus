use async_trait::async_trait;
use plugins::main::imports::Host as ImportHost;
// use plugins::main::toml::{Host as TomlHost, *};
use plugins::main::types::Host as TypeHost;
use std::collections::HashMap;
use wasmtime::component::*;
use wasmtime_wasi::preview2::{WasiCtx, WasiView};

use self::plugins::main::types::{Platform, PluginInfo, ProjectInfo};

use super::PLUGINS_CONFIG;

pub struct PluginRuntimeState {
    pub table: ResourceTable,
    pub ctx: WasiCtx,
    pub metadata: PluginInfo,
    // pub tomls: slab::Slab<TomlValue>,
    pub map: HashMap<String, Vec<u8>>,
}

/// Redacted for now
/// See issue: https://github.com/bytecodealliance/wit-bindgen/issues/817
// impl PluginRuntimeState {
//     pub fn get_toml(&self, value: Resource<Toml>) -> TomlValue {
//         self.tomls
//             .get(value.rep() as usize)
//             .expect("Resource gaurantees existence")
//             .clone()
//     }

//     pub fn set_toml(&mut self, key: Resource<Toml>, value: TomlValue) {
//         *self
//             .tomls
//             .get_mut(key.rep() as usize)
//             .expect("Resource gaurantees existence") = value;
//     }

//     pub fn insert_toml(&mut self, value: TomlValue) -> usize {
//         self.tomls.insert(value)
//     }

//     pub fn new_toml(&mut self, value: TomlValue) -> Resource<Toml> {
//         Resource::new_own(self.insert_toml(value) as u32)
//     }

//     pub fn clone_handle(&mut self, handle: &Resource<Toml>) -> Resource<Toml> {
//         let new_toml = self.get_toml(Resource::new_own(handle.rep()));
//         self.new_toml(new_toml)
//     }
// }

// impl Clone for TomlValue {
//     fn clone(&self) -> Self {
//         match self {
//             TomlValue::String(string) => TomlValue::String(string.clone()),
//             TomlValue::Integer(num) => TomlValue::Integer(*num),
//             TomlValue::Float(float) => TomlValue::Float(*float),
//             TomlValue::Boolean(b) => TomlValue::Boolean(*b),
//             TomlValue::Datetime(d) => TomlValue::Datetime(*d),
//             TomlValue::Array(array) => {
//                 TomlValue::Array(array.iter().map(|f| Resource::new_own(f.rep())).collect())
//             }
//             TomlValue::Table(table) => TomlValue::Table(
//                 table
//                     .iter()
//                     .map(|(key, val)| (key.clone(), Resource::new_own(val.rep())))
//                     .collect(),
//             ),
//         }
//     }
// }

// #[async_trait]
// impl HostToml for PluginRuntimeState {
//     async fn new(&mut self, value: TomlValue) -> wasmtime::Result<Resource<Toml>> {
//         Ok(self.new_toml(value))
//     }
//     async fn get(&mut self, value: Resource<Toml>) -> wasmtime::Result<TomlValue> {
//         Ok(self.get_toml(value)) // We can unwrap because [`Resource`] makes sure the key is always valid
//     }
//     async fn set(&mut self, key: Resource<Toml>, value: TomlValue) -> wasmtime::Result<()> {
//         self.set_toml(key, value);
//         Ok(())
//     }
//     async fn clone(&mut self, key: Resource<Toml>) -> wasmtime::Result<Resource<Toml>> {
//         Ok(self.clone_handle(&key))
//     }

//     fn drop(&mut self, toml: Resource<Toml>) -> wasmtime::Result<()> {
//         // Probably don't need this how it's being used atm but good to check
//         if self.tomls.contains(toml.rep() as usize) {
//             self.tomls.remove(toml.rep() as usize);
//         } else {
//             log::warn!("Tried to drop a dropped resource!");
//         }
//         Ok(())
//     }
// }

// #[async_trait]
// impl TomlHost for PluginRuntimeState {}

#[async_trait]
impl TypeHost for PluginRuntimeState {}

#[async_trait]
impl ImportHost for PluginRuntimeState {
    async fn get_project_info(&mut self) -> wasmtime::Result<ProjectInfo> {
        let application = &PLUGINS_CONFIG.lock().await.application;

        let default_platform = match application.default_platform {
            dioxus_cli_config::Platform::Web => Platform::Web,
            dioxus_cli_config::Platform::Desktop => Platform::Desktop,
            dioxus_cli_config::Platform::Fullstack => Platform::Fullstack,
            dioxus_cli_config::Platform::StaticGeneration => Platform::StaticGeneration,
            _ => return Err(wasmtime::Error::msg("unknown platform")),
        };

        Ok(ProjectInfo { default_platform })
    }

    async fn watch_path(&mut self, path: String) -> wasmtime::Result<()> {
        let mut config = PLUGINS_CONFIG.lock().await;
        let pathbuf = path.into();
        config.web.watcher.watch_path.push(pathbuf);
        Ok(())
    }

    async fn remove_watched_path(&mut self, path: String) -> wasmtime::Result<Result<(), ()>> {
        let mut config = PLUGINS_CONFIG.lock().await;

        let pathbuf: std::path::PathBuf = path.into();

        config.web.watcher.watch_path.retain(|f| f != &pathbuf);

        Ok(Ok(()))
    }

    async fn watched_paths(&mut self) -> wasmtime::Result<Vec<String>> {
        Ok(PLUGINS_CONFIG
            .lock()
            .await
            .web
            .watcher
            .watch_path
            .iter()
            .filter_map(|f| f.to_str().map(ToString::to_string))
            .collect())
    }

    async fn set_data(&mut self, key: String, data: Vec<u8>) -> wasmtime::Result<()> {
        self.map.insert(key, data);
        Ok(())
    }

    async fn get_data(&mut self, key: String) -> wasmtime::Result<Option<Vec<u8>>> {
        Ok(self.map.get(&key).cloned())
    }

    async fn set_config(&mut self, key: String, config: String) -> wasmtime::Result<()> {
        let mut lock = PLUGINS_CONFIG.lock().await;
        let Some(entry) = lock.plugins.plugins.get_mut(&self.metadata.name) else {
            log::warn!("Plugin not initalized correctly! {}", self.metadata.name);
            return Ok(());
        };
        entry.config.insert(key, config);
        Ok(())
    }

    async fn get_config(&mut self, key: String) -> wasmtime::Result<Option<String>> {
        let config = PLUGINS_CONFIG.lock().await;
        let Some(entry) = config.plugins.plugins.get(&self.metadata.name) else {
            log::warn!("Plugin not initalized correctly! {}", self.metadata.name);
            return Ok(None);
        };
        Ok(entry.config.get(&key).cloned())
    }

    async fn log(&mut self, info: String) -> wasmtime::Result<()> {
        log::info!("{info}");
        Ok(())
    }
}

impl WasiView for PluginRuntimeState {
    fn table(&self) -> &ResourceTable {
        &self.table
    }

    fn table_mut(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&self) -> &WasiCtx {
        &self.ctx
    }

    fn ctx_mut(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}

bindgen! ({
    path: "../cli-plugin/wit/plugin.wit",
    async: true
});
