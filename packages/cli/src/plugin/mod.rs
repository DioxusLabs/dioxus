use crate::lock::DioxusLock;
use crate::plugin::interface::{PluginState, PluginWorld};
use crate::PluginConfig;

use slab::Slab;
use std::path::{Path, PathBuf};
use wasmtime::component::*;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::preview2::{self, DirPerms, FilePerms, Table, WasiCtxBuilder};
use wasmtime_wasi::Dir;

use self::convert::ConvertWithState;
use self::interface::plugins::main::imports::PluginInfo;
use self::interface::plugins::main::toml::Toml;

pub mod convert;
pub mod interface;

lazy_static::lazy_static!(
  static ref ENGINE: Engine = {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    Engine::new(&config).unwrap()
  };
);

pub struct Plugins {
    pub plugins: Vec<CliPlugin>,
}

impl Plugins {
    async fn load(config: &PluginConfig) -> wasmtime::Result<Self> {
        let mut plugins = Vec::new();
        for plugin in config.plugins.values() {
            let plugin = load_plugin(&plugin.path).await?;
            plugins.push(plugin);
        }

        let mut dioxus_lock = DioxusLock::load()?;

        dioxus_lock.initialize_new_plugins(&mut plugins).await?;

        Ok(Self { plugins })
    }
}

pub async fn load_plugin(path: impl AsRef<Path>) -> wasmtime::Result<CliPlugin> {
    let path = path.as_ref();
    let component = Component::from_file(&ENGINE, path)?;

    let mut linker = Linker::new(&ENGINE);
    preview2::command::add_to_linker(&mut linker)?;
    PluginWorld::add_to_linker(&mut linker, |state: &mut PluginState| state)?;

    let out_dir =
        std::env::var("CARGO_BUILD_TARGET_DIR").unwrap_or_else(|_| "./target".to_string());
    let sandbox = format!("{}/plugin-sandbox", out_dir);

    std::fs::create_dir_all(&sandbox)?;
    let mut ctx = WasiCtxBuilder::new();
    let ctx_builder = ctx
        .inherit_stderr()
        .inherit_stdin()
        .inherit_stdio()
        .inherit_stdout()
        .preopened_dir(
            Dir::open_ambient_dir(sandbox, wasmtime_wasi::sync::ambient_authority()).unwrap(),
            DirPerms::all(),
            FilePerms::all(),
            ".",
        );
    let table = Table::new();
    let ctx = ctx_builder.build();
    let mut store = Store::new(
        &ENGINE,
        PluginState {
            table,
            ctx,
            tomls: Slab::new(),
        },
    );
    let (bindings, instance) =
        PluginWorld::instantiate_async(&mut store, &component, &linker).await?;

    Ok(CliPlugin {
        bindings,
        instance,
        store,
    })
}

pub struct CliPlugin {
    pub bindings: PluginWorld,
    pub instance: Instance,
    pub store: Store<PluginState>,
}

impl AsMut<PluginState> for CliPlugin {
    fn as_mut(&mut self) -> &mut PluginState {
        self.store.data_mut()
    }
}

impl CliPlugin {
    pub async fn get_default_config(&mut self) -> wasmtime::Result<toml::Value> {
        let default_config = self
            .bindings
            .plugins_main_definitions()
            .call_get_default_config(&mut self.store)
            .await?;
        let t = self
            .store
            .data_mut()
            .get_toml(default_config)
            .convert_with_state(self.store.data_mut())
            .await;
        Ok(t)
    }

    pub async fn apply_config(
        &mut self,
        config: Resource<Toml>,
    ) -> wasmtime::Result<Result<(), ()>> {
        self.bindings
            .plugins_main_definitions()
            .call_apply_config(&mut self.store, config)
            .await
    }

    pub async fn register(&mut self) -> wasmtime::Result<Result<(), ()>> {
        self.bindings
            .plugins_main_definitions()
            .call_register(&mut self.store)
            .await
    }

    pub async fn metadata(&mut self) -> wasmtime::Result<PluginInfo, anyhow::Error> {
        self.bindings
            .plugins_main_definitions()
            .call_metadata(&mut self.store)
            .await
    }

    pub fn clone_handle(&self, handle: &Resource<Toml>) -> Resource<Toml> {
        self.store.data().clone_handle(handle)
    }

    pub async fn get(&mut self, value: Resource<Toml>) -> toml::Value {
        self.store
            .data_mut()
            .get_toml(value)
            .convert_with_state(self.store.data_mut())
            .await
    }

    pub async fn insert_toml(&mut self, value: toml::Value) -> Resource<Toml> {
        let value = value.convert_with_state(self.store.data_mut()).await;
        self.store.data_mut().new_toml(value)
    }

    pub async fn set(&mut self, handle: Resource<Toml>, value: toml::Value) {
        // Should probably check if there is a Toml in the store
        // that is the same as the one we are putting in, currently will just add it to the
        // table
        let value = value.convert_with_state(self.store.data_mut()).await;
        self.store.data_mut().set_toml(handle, value);
    }
}
