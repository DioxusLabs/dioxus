use std::{
    io::{Read, Write},
    path::PathBuf,
};

use mlua::{AsChunk, Lua, Table, ToLuaMulti};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    tools::{app_path, clone_repo},
    CrateConfig,
};

use self::{
    interface::PluginInfo,
    interface::{
        command::PluginCommander, dirs::PluginDirs, fs::PluginFileSystem, log::PluginLogger,
        network::PluginNetwork, os::PluginOS, path::PluginPath,
    },
};

pub mod argument;
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
        let plugin_dir = Self::init_plugin_dir();

        let api = lua.create_table().unwrap();

        api.set("log", PluginLogger).unwrap();
        api.set("command", PluginCommander).unwrap();
        api.set("network", PluginNetwork).unwrap();
        api.set("dirs", PluginDirs).unwrap();
        api.set("fs", PluginFileSystem).unwrap();
        api.set("path", PluginPath).unwrap();
        api.set("os", PluginOS).unwrap();

        lua.globals().set("plugin_lib", api).unwrap();
        lua.globals()
            .set("library_dir", plugin_dir.to_str().unwrap())
            .unwrap();

        let mut index: u32 = 1;
        let mut init_list: Vec<(u32, PathBuf, PluginInfo)> = Vec::new();
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

                    let info = lua.load(&buffer).eval::<PluginInfo>();
                    match info {
                        Ok(info) => {
                            let _ = manager.set(index, info.clone());

                            let dir_name_str = plugin_dir.name().unwrap().to_string();
                            lua.globals().set("current_dir_name", dir_name_str).unwrap();

                            // call `on_init` if file "dcp.json" not exists
                            let dcp_file = plugin_dir.join("dcp.json");
                            if !dcp_file.is_file() {
                                init_list.push((index, dcp_file, info));
                            }

                            index += 1;
                        }
                        Err(_e) => {
                            let dir_name = plugin_dir.file_name().unwrap().to_str().unwrap();
                            log::error!("Plugin '{dir_name}' load failed.");
                        }
                    }
                }
            }
        }

        lua.globals().set("manager", manager).unwrap();

        for (idx, path, info) in init_list {
            let res = lua.load(&format!("manager[{idx}].on_init()")).exec();
            if res.is_ok() {
                let mut file = std::fs::File::create(path).unwrap();
                let value = json!({
                    "name": info.name,
                    "author": info.author,
                    "repository": info.repository,
                    "version": info.version,
                    "generate_time": chrono::Local::now().timestamp(),
                });
                let buffer = serde_json::to_string_pretty(&value).unwrap();
                let buffer = buffer.as_bytes();
                file.write_all(buffer).unwrap();
            }
        }

        Some(Self { lua })
    }

    pub fn on_build_start(&self, crate_config: &CrateConfig, platform: &str) -> anyhow::Result<()> {
        let lua = &self.lua;

        let manager = lua.globals().get::<_, Table>("manager")?;

        let args = lua.create_table()?;
        args.set("name", crate_config.dioxus_config.application.name.clone())?;
        args.set("platform", platform)?;
        args.set("out_dir", crate_config.out_dir.to_str().unwrap())?;
        args.set("asset_dir", crate_config.asset_dir.to_str().unwrap())?;

        for i in 1..(manager.len()? as i32 + 1) {
            let info = manager.get::<i32, PluginInfo>(i)?;
            if let Some(func) = info.build.on_start {
                func.call::<Table, ()>(args.clone())?;
            }
        }

        Ok(())
    }

    pub fn on_build_finish(
        &self,
        crate_config: &CrateConfig,
        platform: &str,
    ) -> anyhow::Result<()> {
        let lua = &self.lua;

        let manager = lua.globals().get::<_, Table>("manager")?;

        let args = lua.create_table()?;
        args.set("name", crate_config.dioxus_config.application.name.clone())?;
        args.set("platform", platform)?;
        args.set("out_dir", crate_config.out_dir.to_str().unwrap())?;
        args.set("asset_dir", crate_config.asset_dir.to_str().unwrap())?;

        for i in 1..(manager.len()? as i32 + 1) {
            let info = manager.get::<i32, PluginInfo>(i)?;
            if let Some(func) = info.build.on_finish {
                func.call::<Table, ()>(args.clone())?;
            }
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
