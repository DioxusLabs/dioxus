use crate::plugin::interface::plugins::main::imports::{Host as ImportHost, Platform};
use crate::plugin::interface::plugins::main::toml::{Host as TomlHost, *};
use crate::plugin::interface::{PluginState, PluginWorld};
use crate::{
    tools::{app_path, clone_repo},
    CrateConfig,
};
use async_trait::async_trait;
use serde_json::json;
use slab::Slab;
use std::path::Path;
use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::Mutex,
};
use wasmtime::component::*;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::preview2::{
    self, DirPerms, FilePerms, Table, WasiCtx, WasiCtxBuilder, WasiView,
};
use wasmtime_wasi::Dir;

pub mod interface;

pub async fn load_plugin(path: impl AsRef<Path>) -> wasmtime::Result<CliPlugin> {
    let path = path.as_ref();
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;

    let component = Component::from_file(&engine, path)?;

    let mut linker = Linker::new(&engine);
    preview2::command::add_to_linker(&mut linker)?;
    PluginWorld::add_to_linker(&mut linker, |state: &mut PluginState| state)?;

    let out_dir = std::env::var("CARGO_BUILD_TARGET_DIR").unwrap_or_else(|_| "./target".to_string());
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
        &engine,
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
    bindings: PluginWorld,
    instance: Instance,
    store: Store<PluginState>,
}

pub struct PluginManager {}

impl PluginManager {
    pub fn init(config: toml::Value) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn on_build_start(
        crate_config: &CrateConfig,
        platform: &crate::cfg::Platform,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn on_build_finish(
        crate_config: &CrateConfig,
        platform: &crate::cfg::Platform,
    ) -> anyhow::Result<()> {
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
