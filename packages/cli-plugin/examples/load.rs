use async_trait::async_trait;
use plugins::main::imports::{Host as ImportHost, Platform};
use plugins::main::toml::{Host as TomlHost, *};
use slab::Slab;
use wasmtime::component::*;
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::preview2::{
    self, DirPerms, FilePerms, Table, WasiCtx, WasiCtxBuilder, WasiView,
};
use wasmtime_wasi::Dir;

struct MyState {
    table: Table,
    ctx: WasiCtx,
    tomls: slab::Slab<TomlValue>,
}

impl Clone for TomlValue {
    fn clone(&self) -> Self {
        match self {
            Self::String(arg0) => Self::String(arg0.clone()),
            Self::Integer(arg0) => Self::Integer(arg0.clone()),
            Self::Float(arg0) => Self::Float(arg0.clone()),
            Self::Array(arg0) => {
                Self::Array(arg0.iter().map(|f| Resource::new_own(f.rep())).collect())
            }
            Self::Table(arg0) => Self::Table(
                arg0.iter()
                    .map(|(key, val)| (key.clone(), Resource::new_own(val.rep())))
                    .collect(),
            ),
        }
    }
}

#[async_trait]
impl HostToml for MyState {
    async fn new(&mut self, value: TomlValue) -> wasmtime::Result<Resource<Toml>> {
        let new_toml = self.tomls.insert(value);
        Ok(Resource::new_own(new_toml as u32))
    }
    async fn get(&mut self, value: Resource<Toml>) -> wasmtime::Result<TomlValue> {
        Ok(self.tomls.get(value.rep() as usize).unwrap().clone()) // We can unwrap because [`Resource`] makes sure the key is always valid
    }
    async fn set(&mut self, key: Resource<Toml>, value: TomlValue) -> wasmtime::Result<()> {
        *self.tomls.get_mut(key.rep() as usize).unwrap() = value;
        Ok(())
    }

    /// Only is called when [`Resource`] detects the [`Toml`] instance is not being called 
    /// iirc
    fn drop(&mut self, toml: Resource<Toml>) -> wasmtime::Result<()> {
        if toml.owned() { // Probably don't need this how it's being used atm but probably good to check
            self.tomls.remove(toml.rep() as usize);
        }
        Ok(())
    }
}

#[async_trait]
impl TomlHost for MyState {}

#[async_trait]
impl ImportHost for MyState {
    async fn output_directory(&mut self) -> wasmtime::Result<String> {
        Ok("output".to_string())
    }

    async fn refresh_browser_page(&mut self) -> wasmtime::Result<()> {
        Ok(())
    }

    async fn refresh_asset(&mut self, _: String, _: String) -> wasmtime::Result<()> {
        Ok(())
    }

    async fn watched_paths(&mut self) -> wasmtime::Result<Vec<String>> {
        Ok(vec!["All of them".into()])
    }

    async fn remove_path(&mut self, _: String) -> wasmtime::Result<bool> {
        Ok(true)
    }

    async fn watch_path(&mut self, _: String) -> wasmtime::Result<()> {
        Ok(())
    }

    async fn get_platform(&mut self) -> wasmtime::Result<Platform> {
        Ok(Platform::Desktop)
    }

    async fn log(&mut self, info: String) -> wasmtime::Result<()> {
        println!("{info}");
        Ok(())
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
    let mut store = Store::new(
        &engine,
        MyState {
            table,
            ctx,
            tomls: Slab::new(),
        },
    );
    let (bindings, _) = PluginWorld::instantiate_async(&mut store, &component, &linker).await?;

    bindings.interface0.call_on_rebuild(&mut store).await?;
    let toml = bindings
        .interface0
        .call_get_default_config(&mut store)
        .await?;
    dbg!(toml.owned());

    bindings
        .interface0
        .call_apply_config(&mut store, toml)
        .await?;

    Ok(())
}

bindgen! ({
    async: true
});
