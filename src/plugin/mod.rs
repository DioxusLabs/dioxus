use std::{fs::create_dir_all, path::PathBuf};

use anyhow::Ok;
use mlua::Lua;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::tools::app_path;

pub mod log;

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

        let plugin_dir = Self::init_plugin_dir();
        for entry in WalkDir::new(plugin_dir).into_iter().filter_map(|e| e.ok()) {
            let plugin_dir = entry.path().to_path_buf();
            if plugin_dir.is_dir() {
                
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
