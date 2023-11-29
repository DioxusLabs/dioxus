use mlua::{FromLua, Function, IntoLua};

pub mod command;
pub mod dirs;
pub mod fs;
pub mod json;
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

    pub inner: PluginInner,

    pub on_init: Option<Function<'lua>>,
    pub build: PluginBuildInfo<'lua>,
    pub serve: PluginServeInfo<'lua>,
}

impl<'lua> FromLua<'lua> for PluginInfo<'lua> {
    fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        let mut res = Self {
            name: String::default(),
            repository: String::default(),
            author: String::default(),
            version: String::from("0.1.0"),

            inner: Default::default(),

            on_init: None,
            build: Default::default(),
            serve: Default::default(),
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

            if let Ok(v) = tab.get::<_, PluginInner>("inner") {
                res.inner = v;
            }

            if let Ok(v) = tab.get::<_, Function>("on_init") {
                res.on_init = Some(v);
            }

            if let Ok(v) = tab.get::<_, PluginBuildInfo>("build") {
                res.build = v;
            }

            if let Ok(v) = tab.get::<_, PluginServeInfo>("serve") {
                res.serve = v;
            }
        }

        Ok(res)
    }
}

impl<'lua> IntoLua<'lua> for PluginInfo<'lua> {
    fn into_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {
        let res = lua.create_table()?;

        res.set("name", self.name.to_string())?;
        res.set("repository", self.repository.to_string())?;
        res.set("author", self.author.to_string())?;
        res.set("version", self.version.to_string())?;

        res.set("inner", self.inner)?;

        if let Some(e) = self.on_init {
            res.set("on_init", e)?;
        }
        res.set("build", self.build)?;
        res.set("serve", self.serve)?;

        Ok(mlua::Value::Table(res))
    }
}

#[derive(Debug, Clone, Default)]
pub struct PluginInner {
    pub plugin_dir: String,
    pub from_loader: bool,
}

impl<'lua> FromLua<'lua> for PluginInner {
    fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        let mut res = Self {
            plugin_dir: String::new(),
            from_loader: false,
        };

        if let mlua::Value::Table(t) = lua_value {
            if let Ok(v) = t.get::<_, String>("plugin_dir") {
                res.plugin_dir = v;
            }
            if let Ok(v) = t.get::<_, bool>("from_loader") {
                res.from_loader = v;
            }
        }
        Ok(res)
    }
}

impl<'lua> IntoLua<'lua> for PluginInner {
    fn into_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {
        let res = lua.create_table()?;

        res.set("plugin_dir", self.plugin_dir)?;
        res.set("from_loader", self.from_loader)?;

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

impl<'lua> IntoLua<'lua> for PluginBuildInfo<'lua> {
    fn into_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {
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

#[derive(Debug, Clone, Default)]
pub struct PluginServeInfo<'lua> {
    pub on_start: Option<Function<'lua>>,
    pub on_rebuild_start: Option<Function<'lua>>,
    pub on_rebuild_end: Option<Function<'lua>>,
    pub on_shutdown: Option<Function<'lua>>,
}

impl<'lua> FromLua<'lua> for PluginServeInfo<'lua> {
    fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        let mut res = Self::default();

        if let mlua::Value::Table(tab) = lua_value {
            if let Ok(v) = tab.get::<_, Function>("on_start") {
                res.on_start = Some(v);
            }
            if let Ok(v) = tab.get::<_, Function>("on_rebuild_start") {
                res.on_rebuild_start = Some(v);
            }
            if let Ok(v) = tab.get::<_, Function>("on_rebuild_end") {
                res.on_rebuild_end = Some(v);
            }
            if let Ok(v) = tab.get::<_, Function>("on_shutdown") {
                res.on_shutdown = Some(v);
            }
        }

        Ok(res)
    }
}

impl<'lua> IntoLua<'lua> for PluginServeInfo<'lua> {
    fn into_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {
        let res = lua.create_table()?;

        if let Some(v) = self.on_start {
            res.set("on_start", v)?;
        }
        if let Some(v) = self.on_rebuild_start {
            res.set("on_rebuild_start", v)?;
        }
        if let Some(v) = self.on_rebuild_end {
            res.set("on_rebuild_end", v)?;
        }

        if let Some(v) = self.on_shutdown {
            res.set("on_shutdown", v)?;
        }

        Ok(mlua::Value::Table(res))
    }
}
