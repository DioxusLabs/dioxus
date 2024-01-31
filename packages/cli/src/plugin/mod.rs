use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::Mutex,
};

use crate::tools::{app_path, clone_repo};
use dioxus_cli_config::CrateConfig;
use mlua::{Lua, Table};
use serde_json::json;

use self::{
    interface::{
        command::PluginCommander, dirs::PluginDirs, fs::PluginFileSystem, log::PluginLogger,
        network::PluginNetwork, os::PluginOS, path::PluginPath, PluginInfo,
    },
    types::PluginConfig,
};

pub mod interface;
mod types;

lazy_static::lazy_static! {
    static ref LUA: Mutex<Lua> = Mutex::new(Lua::new());
}

pub struct PluginManager;

impl PluginManager {
    pub fn init(config: toml::Value) -> anyhow::Result<()> {
        let config = PluginConfig::from_toml_value(config);

        if !config.available {
            return Ok(());
        }

        let lua = LUA.lock().unwrap();

        let manager = lua.create_table().unwrap();
        let name_index = lua.create_table().unwrap();

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
        lua.globals().set("config_info", config.clone())?;

        let mut index: u32 = 1;
        let dirs = std::fs::read_dir(&plugin_dir)?;

        let mut path_list = dirs
            .filter(|v| v.is_ok())
            .map(|v| (v.unwrap().path(), false))
            .collect::<Vec<(PathBuf, bool)>>();
        for i in &config.loader {
            let path = PathBuf::from(i);
            if !path.is_dir() {
                // for loader dir, we need check first, because we need give a error log.
                log::error!("Plugin loader: {:?} path is not a exists directory.", path);
            }
            path_list.push((path, true));
        }

        for entry in path_list {
            let plugin_dir = entry.0.to_path_buf();

            if plugin_dir.is_dir() {
                let init_file = plugin_dir.join("init.lua");
                if init_file.is_file() {
                    let mut file = std::fs::File::open(init_file).unwrap();
                    let mut buffer = String::new();
                    file.read_to_string(&mut buffer).unwrap();

                    let current_plugin_dir = plugin_dir.to_str().unwrap().to_string();
                    let from_loader = entry.1;

                    lua.globals()
                        .set("_temp_plugin_dir", current_plugin_dir.clone())?;
                    lua.globals().set("_temp_from_loader", from_loader)?;

                    let info = lua.load(&buffer).eval::<PluginInfo>();
                    match info {
                        Ok(mut info) => {
                            if name_index.contains_key(info.name.clone()).unwrap_or(false)
                                && !from_loader
                            {
                                // found same name plugin, intercept load
                                log::warn!(
                                    "Plugin {} has been intercepted. [mulit-load]",
                                    info.name
                                );
                                continue;
                            }
                            info.inner.plugin_dir = current_plugin_dir;
                            info.inner.from_loader = from_loader;

                            // call `on_init` if file "dcp.json" not exists
                            let dcp_file = plugin_dir.join("dcp.json");
                            if !dcp_file.is_file() {
                                if let Some(func) = info.clone().on_init {
                                    let result = func.call::<_, bool>(());
                                    match result {
                                        Ok(true) => {
                                            // plugin init success, create `dcp.json` file.
                                            let mut file = std::fs::File::create(dcp_file).unwrap();
                                            let value = json!({
                                                "name": info.name,
                                                "author": info.author,
                                                "repository": info.repository,
                                                "version": info.version,
                                                "generate_time": chrono::Local::now().timestamp(),
                                            });
                                            let buffer =
                                                serde_json::to_string_pretty(&value).unwrap();
                                            let buffer = buffer.as_bytes();
                                            file.write_all(buffer).unwrap();

                                            // insert plugin-info into plugin-manager
                                            if let Ok(index) =
                                                name_index.get::<_, u32>(info.name.clone())
                                            {
                                                let _ = manager.set(index, info.clone());
                                            } else {
                                                let _ = manager.set(index, info.clone());
                                                index += 1;
                                                let _ = name_index.set(info.name, index);
                                            }
                                        }
                                        Ok(false) => {
                                            log::warn!(
                                                "Plugin init function result is `false`, init failed."
                                            );
                                        }
                                        Err(e) => {
                                            log::warn!("Plugin init failed: {e}");
                                        }
                                    }
                                }
                            } else if let Ok(index) = name_index.get::<_, u32>(info.name.clone()) {
                                let _ = manager.set(index, info.clone());
                            } else {
                                let _ = manager.set(index, info.clone());
                                index += 1;
                                let _ = name_index.set(info.name, index);
                            }
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

        Ok(())
    }

    pub fn on_build_start(crate_config: &CrateConfig, platform: &str) -> anyhow::Result<()> {
        let lua = LUA.lock().unwrap();

        if !lua.globals().contains_key("manager")? {
            return Ok(());
        }
        let manager = lua.globals().get::<_, Table>("manager")?;

        let args = lua.create_table()?;
        args.set("name", crate_config.dioxus_config.application.name.clone())?;
        args.set("platform", platform)?;
        args.set("out_dir", crate_config.out_dir().to_str().unwrap())?;
        args.set("asset_dir", crate_config.asset_dir().to_str().unwrap())?;

        for i in 1..(manager.len()? as i32 + 1) {
            let info = manager.get::<i32, PluginInfo>(i)?;
            if let Some(func) = info.build.on_start {
                func.call::<Table, ()>(args.clone())?;
            }
        }

        Ok(())
    }

    pub fn on_build_finish(crate_config: &CrateConfig, platform: &str) -> anyhow::Result<()> {
        let lua = LUA.lock().unwrap();

        if !lua.globals().contains_key("manager")? {
            return Ok(());
        }
        let manager = lua.globals().get::<_, Table>("manager")?;

        let args = lua.create_table()?;
        args.set("name", crate_config.dioxus_config.application.name.clone())?;
        args.set("platform", platform)?;
        args.set("out_dir", crate_config.out_dir().to_str().unwrap())?;
        args.set("asset_dir", crate_config.asset_dir().to_str().unwrap())?;

        for i in 1..(manager.len()? as i32 + 1) {
            let info = manager.get::<i32, PluginInfo>(i)?;
            if let Some(func) = info.build.on_finish {
                func.call::<Table, ()>(args.clone())?;
            }
        }

        Ok(())
    }

    pub fn on_serve_start(crate_config: &CrateConfig) -> anyhow::Result<()> {
        let lua = LUA.lock().unwrap();

        if !lua.globals().contains_key("manager")? {
            return Ok(());
        }
        let manager = lua.globals().get::<_, Table>("manager")?;

        let args = lua.create_table()?;
        args.set("name", crate_config.dioxus_config.application.name.clone())?;

        for i in 1..(manager.len()? as i32 + 1) {
            let info = manager.get::<i32, PluginInfo>(i)?;
            if let Some(func) = info.serve.on_start {
                func.call::<Table, ()>(args.clone())?;
            }
        }

        Ok(())
    }

    pub fn on_serve_rebuild(timestamp: i64, files: Vec<PathBuf>) -> anyhow::Result<()> {
        let lua = LUA.lock().unwrap();

        let manager = lua.globals().get::<_, Table>("manager")?;

        let args = lua.create_table()?;
        args.set("timestamp", timestamp)?;
        let files: Vec<String> = files
            .iter()
            .map(|v| v.to_str().unwrap().to_string())
            .collect();
        args.set("changed_files", files)?;

        for i in 1..(manager.len()? as i32 + 1) {
            let info = manager.get::<i32, PluginInfo>(i)?;
            if let Some(func) = info.serve.on_rebuild {
                func.call::<Table, ()>(args.clone())?;
            }
        }

        Ok(())
    }

    pub fn on_serve_shutdown(crate_config: &CrateConfig) -> anyhow::Result<()> {
        let lua = LUA.lock().unwrap();

        if !lua.globals().contains_key("manager")? {
            return Ok(());
        }
        let manager = lua.globals().get::<_, Table>("manager")?;

        let args = lua.create_table()?;
        args.set("name", crate_config.dioxus_config.application.name.clone())?;

        for i in 1..(manager.len()? as i32 + 1) {
            let info = manager.get::<i32, PluginInfo>(i)?;
            if let Some(func) = info.serve.on_shutdown {
                func.call::<Table, ()>(args.clone())?;
            }
        }

        Ok(())
    }

    pub fn init_plugin_dir() -> PathBuf {
        let app_path = app_path();
        let plugin_path = app_path.join("plugins");
        if !plugin_path.is_dir() {
            log::info!("📖 Start to init plugin library ...");
            let url = "https://github.com/DioxusLabs/cli-plugin-library";
            if let Err(err) = clone_repo(&plugin_path, url) {
                log::error!("Failed to init plugin dir, error caused by {}. ", err);
            }
        }
        plugin_path
    }

    pub fn plugin_list() -> Vec<String> {
        let mut res = vec![];

        if let Ok(lua) = LUA.lock() {
            let list = lua
                .load(mlua::chunk!(
                    local list = {}
                    for key, value in ipairs(manager) do
                        table.insert(list, {name = value.name, loader = value.inner.from_loader})
                    end
                    return list
                ))
                .eval::<Vec<Table>>()
                .unwrap_or_default();
            for i in list {
                let name = i.get::<_, String>("name").unwrap();
                let loader = i.get::<_, bool>("loader").unwrap();

                let text = if loader {
                    format!("{name} [:loader]")
                } else {
                    name
                };
                res.push(text);
            }
        }

        res
    }
}
