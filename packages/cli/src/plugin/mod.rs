use crate::lock::DioxusLock;
use crate::plugin::convert::Convert;
use crate::plugin::interface::{PluginState, PluginWorld};
use crate::{DioxusConfig, PluginConfig};

use slab::Slab;
use std::path::{Path, PathBuf};
use tokio::sync::Mutex;
use wasmtime::component::*;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::preview2::{self, DirPerms, FilePerms, Table, WasiCtxBuilder};
use wasmtime_wasi::Dir;

use self::convert::ConvertWithState;
use self::interface::exports::plugins::main::definitions::Event;
use self::interface::plugins::main::toml::Toml;
use self::interface::plugins::main::types::PluginInfo;

pub mod convert;
pub mod interface;

#[macro_export]
macro_rules! call_plugins {
    (before $event:expr) => {{
        for plugin in $crate::plugin::PLUGINS.lock().await.iter_mut() {
            if plugin.before_event($event).await.is_err() {
                log::warn!(
                    "Could not call Before {:?} on: {}!",
                    $event,
                    plugin.metadata.name
                );
            } else {
                log::info!("Called Before {:?} on: {}", $event, plugin.metadata.name);
            }
        }
    }};
    (after $event:expr) => {{
        for plugin in $crate::plugin::PLUGINS.lock().await.iter_mut() {
            if plugin.after_event($event).await.is_err() {
                log::warn!(
                    "Could not call After {:?} on: {}!",
                    $event,
                    plugin.metadata.name
                );
            } else {
                log::info!("Called After {:?} on: {}", $event, plugin.metadata.name);
            }
        }
    }};
}

lazy_static::lazy_static!(
  static ref ENGINE: Engine = {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    Engine::new(&config).unwrap()
  };

  pub static ref PLUGINS: Mutex<Vec<CliPlugin>> = Default::default();
  pub static ref PLUGINS_CONFIG: Mutex<DioxusConfig> = Default::default();
);

async fn load_plugins(config: &PluginConfig) -> wasmtime::Result<Vec<CliPlugin>> {
    let mut plugins = Vec::new();
    for plugin in config.plugins.values() {
        let plugin = load_plugin(&plugin.path).await?;
        plugins.push(plugin);
    }

    let mut dioxus_lock = DioxusLock::load()?;

    dioxus_lock.initialize_new_plugins(&mut plugins).await?;

    Ok(plugins)
}

pub async fn init_plugins(config: DioxusConfig) -> wasmtime::Result<()> {
    let plugins = load_plugins(&config.plugins).await?;
    *PLUGINS.lock().await = plugins;
    *PLUGINS_CONFIG.lock().await = config;
    Ok(())
}

pub async fn save_plugin_config(bin: PathBuf) -> crate::Result<()> {
    let crate_root = crate::cargo::crate_root()?.join(bin);

    let toml_path = crate_root.join("Dioxus.toml");

    let toml_string = std::fs::read_to_string(&toml_path)?;
    let mut diox_doc: toml_edit::Document = match toml_string.parse() {
        Ok(doc) => doc,
        Err(err) => {
            return Err(crate::Error::Unique(format!(
                "Could not parse Dioxus toml! {}",
                err
            )));
        }
    };

    let watcher_info = toml::Value::try_from(&PLUGINS_CONFIG.lock().await.watcher)
        .expect("Invalid Watcher Config!");
    diox_doc["watcher"] = watcher_info.convert();

    let plugin_info =
        toml::Value::try_from(&PLUGINS_CONFIG.lock().await.plugins).expect("Invalid Plugin Info!");
    diox_doc["plugins"] = plugin_info.convert();

    std::fs::write(toml_path, diox_doc.to_string())?;
    log::info!("✔️  Successfully saved config");
    Ok(())
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
            servers: Slab::new(),
        },
    );
    let (bindings, instance) =
        PluginWorld::instantiate_async(&mut store, &component, &linker).await?;

    let metadata = bindings
        .plugins_main_definitions()
        .call_metadata(&mut store)
        .await?;

    Ok(CliPlugin {
        bindings,
        instance,
        store,
        metadata,
    })
}

pub struct CliPlugin {
    pub bindings: PluginWorld,
    pub instance: Instance,
    pub store: Store<PluginState>,
    pub metadata: PluginInfo,
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
    pub async fn before_event(&mut self, event: Event) -> wasmtime::Result<Result<(), ()>> {
        self.bindings
            .plugins_main_definitions()
            .call_before_event(&mut self.store, event)
            .await
    }
    pub async fn after_event(&mut self, event: Event) -> wasmtime::Result<Result<(), ()>> {
        self.bindings
            .plugins_main_definitions()
            .call_after_event(&mut self.store, event)
            .await
    }

    pub async fn on_watched_paths_change(&mut self, paths: &[String]) -> wasmtime::Result<()> {
        self.bindings
            .plugins_main_definitions()
            .call_on_watched_paths_change(&mut self.store, paths)
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
