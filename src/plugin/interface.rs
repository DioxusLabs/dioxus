use mlua::{FromLua, Function};

pub struct PluginInfo<'lua> {
    name: String,
    repository: String,
    author: String,

    on_init: Option<Function<'lua>>,
    on_load: Option<Function<'lua>>,
    on_build_start: Option<Function<'lua>>,
    on_build_end: Option<Function<'lua>>,
}

impl<'lua> FromLua<'lua> for PluginInfo<'lua> {
    fn from_lua(lua_value: mlua::Value<'lua>, lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        let mut res = Self {
            name: String::default(),
            repository: String::default(),
            author: String::default(),

            on_init: None,
            on_load: None,
            on_build_start: None,
            on_build_end: None,
        };
        if let mlua::Value::Table(tab) = lua_value {
            if let Ok(v) = tab.get::<_, String>("name") {
                res.name = v;
            }
            if let Ok(v) = tab.get::<_, String>("repository") {
                res.repository = v;
            }
            if let Ok(v) = tab.get::<_, String>("author") {
                res.author = v;
            }

            if let Ok(v) = tab.get::<_, Function>("onInit") {
                res.on_init = Some(v);
            }
            if let Ok(v) = tab.get::<_, Function>("onLoad") {
                res.on_load = Some(v);
            }
            if let Ok(v) = tab.get::<_, Function>("onBuildStart") {
                res.on_build_start = Some(v);
            }
            if let Ok(v) = tab.get::<_, Function>("onBuildEnd") {
                res.on_build_end = Some(v);
            }

        }

        Ok(res)
    }
}
