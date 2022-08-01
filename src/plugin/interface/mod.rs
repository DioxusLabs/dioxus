use mlua::{FromLua, Function, ToLua};

pub mod logger;
pub mod command;
pub mod fs;
pub mod download;
pub mod dirs;

#[derive(Debug)]
pub struct PluginInfo<'lua> {
    pub name: String,
    pub repository: String,
    pub author: String,
    pub version: String,

    pub on_init: Option<Function<'lua>>,
    pub on_load: Option<Function<'lua>>,
    pub on_build_start: Option<Function<'lua>>,
    pub on_build_finish: Option<Function<'lua>>,
}

impl<'lua> FromLua<'lua> for PluginInfo<'lua> {
    fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        let mut res = Self {
            name: String::default(),
            repository: String::default(),
            author: String::default(),
            version: String::from("0.1.0"),

            on_init: None,
            on_load: None,
            on_build_start: None,
            on_build_finish: None,
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
            if let Ok(v) = tab.get::<_, String>("version") {
                res.version = v;
            }

            if let Ok(v) = tab.get::<_, Function>("on_init") {
                res.on_init = Some(v);
            }
            if let Ok(v) = tab.get::<_, Function>("on_load") {
                res.on_load = Some(v);
            }
            if let Ok(v) = tab.get::<_, Function>("on_build_start") {
                res.on_build_start = Some(v);
            }
            if let Ok(v) = tab.get::<_, Function>("on_build_finish") {
                res.on_build_finish = Some(v);
            }

        }

        Ok(res)
    }
}

impl<'lua> ToLua<'lua> for PluginInfo<'lua> {
    fn to_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {
        
        let res = lua.create_table()?;
 
        res.set("name", self.name.to_string())?;
        res.set("repository", self.repository.to_string())?;
        res.set("author", self.author.to_string())?;
        res.set("version", self.version.to_string())?;

        if let Some(e) = self.on_init {
            res.set("on_init", e)?;
        }
        if let Some(e) = self.on_load {
            res.set("on_load", e)?;
        }
        if let Some(e) = self.on_build_start {
            res.set("on_build_start", e)?;
        }
        if let Some(e) = self.on_build_finish {
            res.set("on_build_finish", e)?;
        }

        Ok(mlua::Value::Table(res))
    }
}