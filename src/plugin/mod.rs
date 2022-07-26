use std::{fs::create_dir_all, io::Read, path::PathBuf};

use mlua::{Lua, Table};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::tools::{app_path, clone_repo};

use self::{interface::PluginInfo, logger::PluginLogger};

pub mod logger;

mod interface;

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

        lua.globals().set("plugin_logger", PluginLogger).unwrap();

        let plugin_dir = Self::init_plugin_dir();
        let mut index = 0;
        for entry in WalkDir::new(&plugin_dir).into_iter().filter_map(|e| e.ok()) {
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

        lua.globals()
            .set("package.path", format!("{}", plugin_dir.display())).unwrap();

        lua.globals().set("manager", manager).unwrap();

        Some(Self { lua })
    }

    pub fn load_all_plugins(&self) -> anyhow::Result<()> {
        let lua = &self.lua;
        let manager = lua.globals().get::<_, Table>("manager")?;
        println!("{:?}", manager.len());
        for i in 0..(manager.len()? as i32) {
            let v = manager.get::<i32, PluginInfo>(i)?;
            println!("{:?}", v.name);
            let code = format!("manager[{i}].onLoad()");
            lua.load(&code).exec()?;
        }
        Ok(())
    }

    fn init_plugin_dir() -> PathBuf {
        log::info!("📖 Start to init plugin library ...");

        let app_path = app_path();
        let plugin_path = app_path.join("plugins");
        if !plugin_path.is_dir() {
            let url = "https://github.com/DioxusLabs/cli-plugin-library";
            clone_repo(&plugin_path, url).unwrap();
        }
        plugin_path
    }
}
