use mlua::{FromLua, Function, ToLua};

pub mod command;
pub mod dirs;
pub mod fs;
pub mod log;
pub mod network;
pub mod os;
pub mod path;

#[derive(Debug, Clone)]
pub struct PluginInfo<'lua> {
    pub name: String,
    pub repository: String,
    pub author: String,
    pub version: String,

    pub on_init: Option<Function<'lua>>,
    pub build: PluginBuildInfo<'lua>,
}

impl<'lua> FromLua<'lua> for PluginInfo<'lua> {
    fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        let mut res = Self {
            name: String::default(),
            repository: String::default(),
            author: String::default(),
            version: String::from("0.1.0"),

            on_init: None,
            build: Default::default(),
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
        res.set("build", self.build)?;

        Ok(mlua::Value::Table(res))
    }
}

#[derive(Debug, Clone, Default)]
pub struct PluginBuildInfo<'lua> {
    pub on_start: Option<Function<'lua>>,
    pub on_finish: Option<Function<'lua>>,
}

impl<'lua> FromLua<'lua> for PluginBuildInfo<'lua> {
    fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        let mut res = Self {
            on_start: None,
            on_finish: None,
        };

        if let mlua::Value::Table(t) = lua_value {
            if let Ok(v) = t.get::<_, Function>("on_start") {
                res.on_start = Some(v);
            }
            if let Ok(v) = t.get::<_, Function>("on_finish") {
                res.on_finish = Some(v);
            }
        }

        Ok(res)
    }
}

impl<'lua> ToLua<'lua> for PluginBuildInfo<'lua> {
    fn to_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {

        let res = lua.create_table()?;


        if let Some(v) = self.on_start {
            res.set("on_start", v)?;
        }

        if let Some(v) = self.on_finish {
            res.set("on_finish", v)?;
        }

        Ok(mlua::Value::Table(res))
    }
}