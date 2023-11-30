

use async_trait::async_trait;
use dioxus_cli::plugin::interface::plugins::main::imports::{Host as ImportHost, Platform};
use dioxus_cli::plugin::interface::plugins::main::toml::{Host as TomlHost, *};
use slab::Slab;
use wasmtime::component::*;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::preview2::{
    self, DirPerms, FilePerms, Table, WasiCtx, WasiCtxBuilder, WasiView,
};
use wasmtime_wasi::Dir;
use dioxus_cli::plugin::interface::{PluginWorld, PluginState};


#[tokio::test]
async fn load_plugin() -> wasmtime::Result<()> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;

    let component = Component::from_file(&engine, "./output.wasm")?;

    let mut linker = Linker::new(&engine);
    preview2::command::add_to_linker(&mut linker)?;
    PluginWorld::add_to_linker(&mut linker, |state: &mut PluginState| state)?;

    let sandbox = "./plugin-sandbox";
    std::fs::create_dir_all(sandbox)?;
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
    let (bindings, _) = PluginWorld::instantiate_async(&mut store, &component, &linker).await?;

    // bindings.interface0.call_on_rebuild(&mut store).await?;
    // let toml = bindings
    //     .interface0
    //     .call_get_default_config(&mut store)
    //     .await?;
    // dbg!(toml.owned());

    // bindings
    //     .interface0
    //     .call_apply_config(&mut store, toml)
    //     .await?;

    Ok(())
}
