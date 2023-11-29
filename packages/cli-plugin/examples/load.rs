use crate::plugins::main::imports::Host;
use async_trait::async_trait;
use wasmtime::component::*;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::preview2::{
    self, DirPerms, FilePerms, Table, WasiCtx, WasiCtxBuilder, WasiView,
};
use wasmtime_wasi::Dir;

struct MyState {
    table: Table,
    ctx: WasiCtx,
}

#[async_trait]
impl Host for MyState {
    async fn output_directory(&mut self) -> wasmtime::Result<String> {
        Ok("output".to_string())
    }

    async fn reload_browser(&mut self) -> wasmtime::Result<()> {
        Ok(())
    }

    async fn refresh_asset(&mut self, _: String, _: String) -> wasmtime::Result<()> {
        Ok(())
    }

    async fn watch_path(&mut self, _: String) -> wasmtime::Result<()> {
        Ok(())
    }

    async fn get_platform(&mut self) -> wasmtime::Result<String> { // TODO make an enum
      Ok("".to_string())
    }
}

impl WasiView for MyState {
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

#[tokio::main]
async fn main() -> wasmtime::Result<()> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;

    let component = Component::from_file(&engine, "./output.wasm")?;

    let mut linker = Linker::new(&engine);
    preview2::command::add_to_linker(&mut linker)?;
    PluginWorld::add_to_linker(&mut linker, |state: &mut MyState| state)?;

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
    let mut store = Store::new(&engine, MyState { table, ctx });
    let (bindings, _) = PluginWorld::instantiate_async(&mut store, &component, &linker).await?;

    bindings.interface0.call_on_rebuild(&mut store).await?;

    Ok(())
}

bindgen! ({
    async: true
});
