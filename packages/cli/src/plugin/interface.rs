use async_trait::async_trait;
use ext_toml::value::Map;
use plugins::main::imports::{Host as ImportHost, Platform};
use plugins::main::toml::{Host as TomlHost, *};
use wasmtime::component::*;
use wasmtime_wasi::preview2::{Table, WasiCtx, WasiView};

pub struct PluginState {
    pub table: Table,
    pub ctx: WasiCtx,
    pub tomls: slab::Slab<TomlValue>,
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

use toml as ext_toml;

impl From<ext_toml::value::Offset> for Offset {
    fn from(value: ext_toml::value::Offset) -> Self {
        match value {
            ext_toml::value::Offset::Z => Offset::Z,
            ext_toml::value::Offset::Custom { hours, minutes } => Offset::Custom((hours, minutes)),
        }
    }
}

impl From<ext_toml::value::Time> for Time {
    fn from(value: ext_toml::value::Time) -> Self {
        let ext_toml::value::Time {
            hour,
            minute,
            second,
            nanosecond,
        } = value;

        Time {
            hour,
            minute,
            second,
            nanosecond,
        }
    }
}

impl From<ext_toml::value::Date> for Date {
    fn from(value: ext_toml::value::Date) -> Self {
        let ext_toml::value::Date { year, month, day } = value;

        Date { year, month, day }
    }
}

impl From<ext_toml::value::Datetime> for Datetime {
    fn from(value: ext_toml::value::Datetime) -> Self {
        let ext_toml::value::Datetime { date, time, offset } = value;

        Datetime {
            date: date.map(Into::into),
            time: time.map(Into::into),
            offset: offset.map(Into::into),
        }
    }
}

#[async_trait]
pub trait ConvertWithState<T> {
  async fn convert_with_state(self, state: &mut PluginState) -> T;
}

pub trait Convert<T> {
  fn convert(self) -> T;
}

impl<T, U> Convert<Option<T>> for Option<U> where U: Convert<T> {
  fn convert(self) -> Option<T> {
    self.map(Convert::convert)
  }
}

impl Convert<ext_toml::value::Datetime> for Datetime {
  fn convert(self) -> ext_toml::value::Datetime {
    let Datetime { date, time, offset } = self;
    ext_toml::value::Datetime {
      date: date.convert(),
      time: time.convert(),
      offset: offset.convert(),
    }
  }
}

impl Convert<ext_toml::value::Time> for Time {
  fn convert(self) -> ext_toml::value::Time {
    let Time { hour, minute, second, nanosecond } = self;
    ext_toml::value::Time {hour, minute, second, nanosecond}
  }
}

impl Convert<ext_toml::value::Date> for Date {
  fn convert(self) -> ext_toml::value::Date {
    let Date { year, month, day } = self;
    ext_toml::value::Date { year, month, day }
  }
}

impl Convert<ext_toml::value::Offset> for Offset {
  fn convert(self) -> ext_toml::value::Offset {
    match self {
      Offset::Z => ext_toml::value::Offset::Z,
      Offset::Custom((hours, minutes)) => ext_toml::value::Offset::Custom { hours, minutes },
    }
  }
}

use ext_toml::Value as Value;
#[async_trait]
impl ConvertWithState<Value> for TomlValue {
    async fn convert_with_state(self, state: &mut PluginState) -> Value {
        match self {
            TomlValue::String(string) => Value::String(string),
            TomlValue::Integer(int) => Value::Integer(int),
            TomlValue::Float(float) => Value::Float(float),
            TomlValue::Boolean(b) => Value::Boolean(b),
            TomlValue::Datetime(datetime) => Value::Datetime(datetime.convert()),
            TomlValue::Array(array) => {
              let mut new_array = Vec::with_capacity(array.len());
              for item in array.into_iter() {
                new_array.push(state.get(item).await.unwrap().convert_with_state(state).await)
              }
              Value::Array(new_array)
            }
            TomlValue::Table(t) => {
              let mut table = Map::new();
              for (key, value) in t {
                  let converted = state.get(value).await.unwrap().convert_with_state(state).await; 
                  table.insert(key, converted);
              }
              Value::Table(table)
          }
        }
    }
}

#[async_trait]
impl HostToml for PluginState {
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
    async fn clone(&mut self, key: Resource<Toml>) -> wasmtime::Result<Resource<Toml>> {
        Ok(Resource::new_own(key.rep()))
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
