use async_trait::async_trait;
use plugins::main::imports::Host as ImportHost;
use plugins::main::toml::{Host as TomlHost, *};
use plugins::main::types::Host as TypeHost;
use std::collections::HashMap;
use wasmtime::component::*;
use wasmtime_wasi::preview2::{Table, WasiCtx, WasiView};

use self::plugins::main::types::{Platform, ProjectInfo};

use super::PLUGINS_CONFIG;

pub struct PluginRuntimeState {
    pub table: Table,
    pub ctx: WasiCtx,
    pub tomls: slab::Slab<TomlValue>,
    pub map: HashMap<String, Vec<u8>>,
}

impl PluginRuntimeState {
    pub fn get_toml(&mut self, value: Resource<Toml>) -> TomlValue {
        self.tomls.get(value.rep() as usize).unwrap().clone()
    }

    pub fn set_toml(&mut self, key: Resource<Toml>, value: TomlValue) {
        *self.tomls.get_mut(key.rep() as usize).unwrap() = value;
    }

    pub fn insert_toml(&mut self, value: TomlValue) -> usize {
        self.tomls.insert(value)
    }

    pub fn new_toml(&mut self, value: TomlValue) -> Resource<Toml> {
        Resource::new_own(self.insert_toml(value) as u32)
    }

    // Get reference so we know that a table is being kept up with
    // Probably redundant, but will probably be better if need borrow checking later
    pub fn clone_handle(&self, handle: &Resource<Toml>) -> Resource<Toml> {
        Resource::new_own(handle.rep())
    }
}

impl Clone for TomlValue {
    fn clone(&self) -> Self {
        match self {
            TomlValue::String(string) => TomlValue::String(string.clone()),
            TomlValue::Integer(num) => TomlValue::Integer(*num),
            TomlValue::Float(float) => TomlValue::Float(*float),
            TomlValue::Boolean(b) => TomlValue::Boolean(*b),
            TomlValue::Datetime(d) => TomlValue::Datetime(*d),
            TomlValue::Array(array) => {
                TomlValue::Array(array.iter().map(|f| Resource::new_own(f.rep())).collect())
            }
            TomlValue::Table(table) => TomlValue::Table(
                table
                    .iter()
                    .map(|(key, val)| (key.clone(), Resource::new_own(val.rep())))
                    .collect(),
            ),
        }
    }
}

#[async_trait]
impl HostToml for PluginRuntimeState {
    async fn new(&mut self, value: TomlValue) -> wasmtime::Result<Resource<Toml>> {
        Ok(self.new_toml(value))
    }
    async fn get(&mut self, value: Resource<Toml>) -> wasmtime::Result<TomlValue> {
        Ok(self.get_toml(value)) // We can unwrap because [`Resource`] makes sure the key is always valid
    }
    async fn set(&mut self, key: Resource<Toml>, value: TomlValue) -> wasmtime::Result<()> {
        self.set_toml(key, value);
        Ok(())
    }
    async fn clone(&mut self, key: Resource<Toml>) -> wasmtime::Result<Resource<Toml>> {
        Ok(self.clone_handle(&key))
    }

    /// Only is called when [`Resource`] detects the [`Toml`] instance is not being called
    /// iirc
    fn drop(&mut self, toml: Resource<Toml>) -> wasmtime::Result<()> {
        if toml.owned() {
            // Probably don't need this how it's being used atm but probably good to check
            self.tomls.remove(toml.rep() as usize);
        }
        Ok(())
    }
}

#[async_trait]
impl TomlHost for PluginRuntimeState {}

#[async_trait]
impl TypeHost for PluginRuntimeState {}

#[async_trait]
impl ImportHost for PluginRuntimeState {
    async fn get_project_info(&mut self) -> wasmtime::Result<ProjectInfo> {
        let application = &PLUGINS_CONFIG.lock().await.application;

        let has_output_directory = application.out_dir.is_some();
        let has_assets_directory = application.asset_dir.is_some();
        let default_platform = match application.default_platform {
            crate::cfg::Platform::Web => Platform::Web,
            crate::cfg::Platform::Desktop => Platform::Desktop,
        };

        Ok(ProjectInfo {
            has_output_directory,
            has_assets_directory,
            default_platform,
        })
    }

    async fn watch_path(&mut self, path: String) -> wasmtime::Result<()> {
        let mut config = PLUGINS_CONFIG.lock().await;
        let pathbuf = path.into();
        match config.watcher.watch_path.as_mut() {
            Some(watched_paths) => watched_paths.push(pathbuf),
            None => config.watcher.watch_path = Some(vec![pathbuf]),
        }
        Ok(())
    }

    async fn remove_watched_path(&mut self, path: String) -> wasmtime::Result<Result<(), ()>> {
        let mut config = PLUGINS_CONFIG.lock().await;

        let Some(paths) = config.watcher.watch_path.as_mut() else {
            return Ok(Err(()));
        };

        let pathbuf: std::path::PathBuf = path.into();

        let Some(index) = paths.iter().position(|f| f == &pathbuf) else {
            return Ok(Err(()));
        };

        paths.remove(index);

        Ok(Ok(()))
    }

    async fn watched_paths(&mut self) -> wasmtime::Result<Vec<String>> {
        Ok(
            match PLUGINS_CONFIG.lock().await.watcher.watch_path.as_ref() {
                Some(paths) => paths
                    .iter()
                    .map(|f| f.to_str().unwrap_or_default().to_string())
                    .collect(),
                None => vec![],
            },
        )
    }

    async fn set_data(&mut self, key: String, data: Vec<u8>) -> wasmtime::Result<()> {
        self.map.insert(key, data);
        Ok(())
    }

    async fn get_data(&mut self, key: String) -> wasmtime::Result<Option<Vec<u8>>> {
        Ok(self.map.get(&key).cloned())
    }

    async fn log(&mut self, info: String) -> wasmtime::Result<()> {
        println!("{info}");
        Ok(())
    }
}

impl WasiView for PluginRuntimeState {
    fn table(&self) -> &Table {
        &self.table
    }

    fn table_mut(&mut self) -> &mut Table {
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
