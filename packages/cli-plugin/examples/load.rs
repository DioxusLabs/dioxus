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
impl PluginWorldImports for MyState {
    async fn output_directory(&mut self) -> wasmtime::Result<String> {
        Ok("output".to_string())
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
    // let bytes = std::fs::read("/Users/evanalmloff/Desktop/Github/dioxus/packages/cli-plugin/examples/dioxus_cli_plugin_test.wasm")?;
    // let component = ComponentEncoder::default()
    //     .validate(false)
    //     .module(bytes.as_slice())?
    //     .adapter(
    //         "wasi_snapshot_preview1",
    //         include_bytes!("../wasi_snapshot_preview1.wasm",),
    //     )
    //     .unwrap()
    //     .encode()?;

    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;

    let component = Component::from_file(&engine, "./output.wasm")?;

    // Instantiation of bindings always happens through a `Linker`.
    // Configuration of the linker is done through a generated `add_to_linker`
    // method on the bindings structure.
    //
    // Note that the closure provided here is a projection from `T` in
    // `Store<T>` to `&mut U` where `U` implements the `HelloWorldImports`
    // trait. In this case the `T`, `MyState`, is stored directly in the
    // structure so no projection is necessary here.
    let mut linker = Linker::new(&engine);
    preview2::command::add_to_linker(&mut linker)?;
    PluginWorld::add_to_linker(&mut linker, |state: &mut MyState| state)?;

    // As with the core wasm API of Wasmtime instantiation occurs within a
    // `Store`. The bindings structure contains an `instantiate` method which
    // takes the store, component, and linker. This returns the `bindings`
    // structure which is an instance of `HelloWorld` and supports typed access
    // to the exports of the component.
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
