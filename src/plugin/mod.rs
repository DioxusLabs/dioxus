use std::{io::Read, path::PathBuf};

use mlua::{Lua, Table};
use serde::{Deserialize, Serialize};

use crate::tools::{app_path, clone_repo};

use self::{
    interface::PluginInfo,
    interface::{
        command::PluginCommander, dirs::PluginDirs, download::PluginDownloader,
        fs::PluginFileSystem, logger::PluginLogger,
    },
};

pub mod interface;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub available: bool,
    pub required: Vec<String>,
}

pub struct PluginManager {
    lua: Lua,
}

impl PluginManager {
    pub fn init(config: &PluginConfig) -> Option<Self> {
        if !config.available {
            return None;
        }

        let lua = Lua::new();

        let manager = lua.create_table().unwrap();

        lua.globals().set("PLUGIN_LOGGER", PluginLogger).unwrap();
        lua.globals()
            .set("PLUGIN_COMMAND", PluginCommander)
            .unwrap();
        lua.globals().set("PLUGIN_FS", PluginFileSystem).unwrap();
        lua.globals()
            .set("PLUGIN_DOWNLOAD", PluginDownloader)
            .unwrap();
        lua.globals().set("PLUGIN_DIRS", PluginDirs).unwrap();

        let plugin_dir = Self::init_plugin_dir();
        let mut index = 1;
        for entry in std::fs::read_dir(&plugin_dir).ok()? {
            if entry.is_err() {
                continue;
            }
            let entry = entry.unwrap();
            let plugin_dir = entry.path().to_path_buf();
            if plugin_dir.is_dir() {
                let init_file = plugin_dir.join("init.lua");
                if init_file.is_file() {
                    let mut file = std::fs::File::open(init_file).unwrap();
                    let mut buffer = String::new();
                    file.read_to_string(&mut buffer).unwrap();
                    let info = lua.load(&buffer).eval::<PluginInfo>().unwrap();
                    let _ = manager.set(index, info);
                    index += 1;
                }
            }
        }

        lua.globals()
            .set("LIBDIR", plugin_dir.join("library").to_str().unwrap())
            .unwrap();
        lua.globals().set("manager", manager).unwrap();

        Some(Self { lua })
    }

    pub fn load_all_plugins(&self) -> anyhow::Result<()> {
        let lua = &self.lua;
        let manager = lua.globals().get::<_, Table>("manager")?;
        for i in 1..(manager.len()? as i32 + 1) {
            let v = manager.get::<i32, PluginInfo>(i)?;
            println!("{v:?}");
            let code = format!("manager[{i}].on_load()");
            lua.load(&code).exec()?;
        }
        Ok(())
    }

    fn init_plugin_dir() -> PathBuf {
        let app_path = app_path();
        let plugin_path = app_path.join("plugins");
        if !plugin_path.is_dir() {
            log::info!("📖 Start to init plugin library ...");
            let url = "https://github.com/DioxusLabs/cli-plugin-library";
            clone_repo(&plugin_path, url).unwrap();
        }
        plugin_path
    }
}
