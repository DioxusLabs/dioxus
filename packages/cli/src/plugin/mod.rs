use crate::plugin::interface::{PluginState, PluginWorld};

use slab::Slab;
use std::path::Path;
use wasmtime::component::*;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::preview2::{self, DirPerms, FilePerms, Table, WasiCtxBuilder};
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
    pub bindings: PluginWorld,
    pub instance: Instance,
    pub store: Store<PluginState>,
}
