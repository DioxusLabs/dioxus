use std::collections::HashMap;

use mlua::ToLua;

use super::LUA;

#[derive(Debug, Clone)]
pub struct PluginConfig {
    available: bool,
    config_info: HashMap<String, HashMap<String, Value>>,
}

impl<'lua> ToLua<'lua> for PluginConfig {
    fn to_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {
        let table = lua.create_table()?;

        table.set("available", self.available)?;

        let config_info = lua.create_table()?;

        for (name, data) in self.config_info {
            config_info.set(name, data)?;
        }

        table.set("config_info", config_info)?;

        Ok(mlua::Value::Table(table))
    }
}

impl PluginConfig {
    pub fn from_toml_value(val: toml::Value) -> Self {
        if let toml::Value::Table(tab) = val {
            let available = tab
                .get::<_>("available")
                .unwrap_or(&toml::Value::Boolean(true));
            let available = available.as_bool().unwrap_or(true);

            let mut config_info = HashMap::new();

            let lua = LUA.lock().unwrap();

            for (name, value) in tab {}

            Self {
                available,
                config_info,
            }
        } else {
            Self {
                available: false,
                config_info: HashMap::new(),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<Value>),
    Table(HashMap<String, Value>),
}

impl Value {
    pub fn from_toml(origin: toml::Value) -> Self {
        match origin {
            cargo_toml::Value::String(s) => Value::String(s),
            cargo_toml::Value::Integer(i) => Value::Integer(i),
            cargo_toml::Value::Float(f) => Value::Float(f),
            cargo_toml::Value::Boolean(b) => Value::Boolean(b),
            cargo_toml::Value::Datetime(d) => Value::String(d.to_string()),
            cargo_toml::Value::Array(a) => {
                let mut v = vec![];
                for i in a {
                    v.push(Value::from_toml(i));
                }
                Value::Array(v)
            },
            cargo_toml::Value::Table(t) => {
                let mut h = HashMap::new();
                for (n, v) in t {
                    h.insert(n, Value::from_toml(v));
                }
                Value::Table(h)
            },
        }
    }
}

impl<'lua> ToLua<'lua> for Value {
    fn to_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {
        Ok(
            match self {
                Value::String(s) => mlua::Value::String(lua.create_string(&s)?),
                Value::Integer(i) => mlua::Value::Integer(i),
                Value::Float(f) => mlua::Value::Number(f),
                Value::Boolean(b) => mlua::Value::Boolean(b),
                Value::Array(a) => {
                    let table = lua.create_table()?;
                    for (i, v) in a.iter().enumerate() {
                        table.set(i, v.clone())?;
                    }
                    mlua::Value::Table(table)
                },
                Value::Table(t) => {
                    let table = lua.create_table()?;
                    for (i, v) in t.iter() {
                        table.set(i.clone(), v.clone())?;
                    }
                    mlua::Value::Table(table)
                },
            }
        )
    }
}