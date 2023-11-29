use mlua::{LuaSerdeExt, UserData};
use serde::Serialize;

pub struct PluginJson;
impl UserData for PluginJson {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("decode", |lua, value: String| {
            let s = value.as_bytes();
            let json_value = serde_json::from_slice::<serde_json::Value>(s)
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            lua.to_value(&json_value)
        });
        methods.add_function("encode", |lua, value: mlua::Value| {
            let mut buf = Vec::new();
            value
                .serialize(&mut serde_json::Serializer::new(&mut buf))
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            lua.create_string(&buf).map(mlua::Value::String)
        });
    }
}
