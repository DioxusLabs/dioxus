use std::{fs::create_dir_all, path::PathBuf, io::Read};

use mlua::Lua;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::tools::app_path;

use self::{log::PluginLogger, interface::PluginInfo};

pub mod log;

mod interface;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    available: bool,
    required: Vec<String>,
}

pub struct PluginManager {
    lua: Lua,
}

impl PluginManager {
    pub fn init(config: &PluginConfig) -> Option<Self> {
        if config.available {
            return None;
        }

        let lua = Lua::new();

        let manager = lua.create_table().ok()?;

        lua.globals().set("plugin_logger", PluginLogger).unwrap();

        let plugin_dir = Self::init_plugin_dir();
        let mut index = 0;
        for entry in WalkDir::new(plugin_dir).into_iter().filter_map(|e| e.ok()) {
            let plugin_dir = entry.path().to_path_buf();
            if plugin_dir.is_dir() {
                let init_file = plugin_dir.join("init.lua");
                if init_file.is_file() {
                    let mut file = std::fs::File::open(init_file).unwrap();
                    let mut buffer = String::new();
                    file.read_to_string(&mut buffer).unwrap();
                    let info = lua.load(&buffer).eval::<mlua::Value>().unwrap();
                    let _ = manager.set(index, info);
                }
                index += 1;
            }
        }

        lua.globals().set("manager", manager).ok()?;

        Some(Self { lua })
    }

    fn init_plugin_dir() -> PathBuf {
        let app_path = app_path();
        let plugin_path = app_path.join("plugins");
        if !plugin_path.is_dir() {
            create_dir_all(&plugin_path).unwrap();
        }
        plugin_path
    }
}
