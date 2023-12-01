use async_trait::async_trait;
use plugins::main::imports::{Host as ImportHost, Platform};
use plugins::main::toml::{Host as TomlHost, *};
use wasmtime::component::*;
use wasmtime_wasi::preview2::{Table, WasiCtx, WasiView};

pub struct PluginState {
    pub table: Table,
    pub ctx: WasiCtx,
    pub tomls: slab::Slab<TomlValue>,
}

impl PluginState {
    pub fn get_toml(&mut self, value: Resource<Toml>) -> TomlValue {
        self.tomls.get(value.rep() as usize).unwrap().clone()
    }

    pub fn set_toml(&mut self, key: Resource<Toml>, value: TomlValue) {
        *self.tomls.get_mut(key.rep() as usize).unwrap() = value;
    }

    pub fn insert_toml(&mut self, value: TomlValue) -> usize {
        self.tomls.insert(value)
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
impl HostToml for PluginState {
    async fn new(&mut self, value: TomlValue) -> wasmtime::Result<Resource<Toml>> {
        Ok(Resource::new_own(self.insert_toml(value) as u32))
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
impl TomlHost for PluginState {}

#[async_trait]
impl ImportHost for PluginState {
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

    async fn remove_path(&mut self, _: String) -> wasmtime::Result<Result<(), ()>> {
        Ok(Ok(()))
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

impl WasiView for PluginState {
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
