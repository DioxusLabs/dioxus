use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};

use mlua::{chunk, Lua, Table};

use crate::{
    cfg::Platform,
    crate_root,
    tools::{app_path, clone_repo},
    CrateConfig, DioxusConfig,
};

use self::{
    interface::{
        command::PluginCommander, dirs::PluginDirs, fs::PluginFileSystem, json::PluginJson,
        log::PluginLogger, network::PluginNetwork, os::PluginOS, path::PluginPath, PluginInfo,
    },
    status::{get_plugin_status, set_plugin_status, PluginStatus},
};

pub const CORE_LIBRARY_VERSION: &str = "0.4.0";

pub mod interface;
pub mod status;
pub mod types;

lazy_static::lazy_static! {
    static ref LUA: Mutex<Lua> = Mutex::new(Lua::new());
}

pub struct PluginManager;

impl PluginManager {
    pub fn get_plugin_dir() -> Option<PathBuf> {
        let crate_root = crate_root().unwrap();
        let plugins_dir = crate_root.join(".dioxus").join("plugins");
        if plugins_dir.join("core").is_dir() {
            return Some(plugins_dir);
        }
        None
    }

    pub fn init(config: DioxusConfig) -> anyhow::Result<()> {
        // if plugin is unavailable (get_plugin_dir return None), then stop init pluginManager
        let plugin_dir = if let Some(v) = Self::get_plugin_dir() {
            v
        } else {
            return Ok(());
        };

        let lua = LUA.lock().expect("Lua runtime load failed");

        // if CLI support core library version != current library version, give warnning
        let version_file = plugin_dir.join("core").join("version.lua");
        if !version_file.is_file() {
            return Ok(());
        }
        let version = lua
            .load(&std::fs::read_to_string(version_file).unwrap())
            .eval::<String>()
            .expect("Load version failed.");
        if version != CORE_LIBRARY_VERSION {
            log::warn!("Core library is not same with CLI version, maybe have compatible problem!");
            log::warn!("You can use `dioxus plugin upgrade core` command upgrade it.");
            // make this warnning remain 3 seconds
            std::thread::sleep(std::time::Duration::from_secs(3));
        }

        let manager = lua.create_table().expect("Lua runtime init failed");
        let name_index = lua.create_table().expect("Lua runtime init failed");

        let api = lua.create_table().unwrap();

        api.set("log", PluginLogger)
            .expect("Plugin: `log` library init faield");
        api.set("command", PluginCommander)
            .expect("Plugin: `command` library init faield");
        api.set("network", PluginNetwork)
            .expect("Plugin: `network` library init faield");
        api.set("dirs", PluginDirs)
            .expect("Plugin: `dirs` library init faield");
        api.set("fs", PluginFileSystem)
            .expect("Plugin: `fs` library init faield");
        api.set("path", PluginPath)
            .expect("Plugin: `path` library init faield");
        api.set("os", PluginOS)
            .expect("Plugin: `os` library init faield");
        api.set("json", PluginJson)
            .expect("Plugin `json` library init failed");

        lua.globals()
            .set("plugin_lib", api)
            .expect("Plugin: library startup failed");
        lua.globals().set("plugin_config", config.clone())?;

        // auto-load library_dir
        let core_path = plugin_dir.join("core");
        let library_dir = core_path.to_str().unwrap();
        lua.load(chunk!(package.path = $library_dir.."/?.lua"))
            .exec()?;

        let mut index: u32 = 1;
        let dirs = std::fs::read_dir(&plugin_dir)?;

        let path_list = dirs
            .filter(|v| v.is_ok())
            .map(|v| (v.unwrap().path(), false))
            .collect::<Vec<(PathBuf, bool)>>();

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

                    let info = lua.load(&buffer).eval::<PluginInfo>();
                    match info {
                        Ok(mut info) => {
                            if name_index.contains_key(info.name.clone()).unwrap_or(false)
                                && !from_loader
                            {
                                // found same name plugin, intercept load
                                log::warn!(
                                    "Plugin `{}` has been intercepted. [mulit-load]",
                                    info.name
                                );
                                continue;
                            }
                            info.inner.plugin_dir = current_plugin_dir;

                            // call `on_init` if plugin info not in the `Plugin.lock` file
                            let mut plugin_status = get_plugin_status(&info.name);
                            if plugin_status.is_some() {
                                let status = plugin_status.clone().unwrap();
                                if status.version != info.version {
                                    log::warn!("Plugin locked version is `{0}` but loading version is `{1}`", status.version, info.version);
                                    log::warn!("Do you want to re-init plugin? (Y/N)");
                                    log::warn!("If you choose `N` this warning has always existed until you re-init it.");
                                    let mut input = String::new();
                                    let _ = std::io::stdin().read_line(&mut input);
                                    if input.trim().to_uppercase() == "Y" {
                                        plugin_status = None;
                                    }
                                }
                            }

                            // if plugin don't have init info, then call init function.
                            if plugin_status.is_none() {
                                if let Some(func) = info.clone().on_init {
                                    let result = func.call::<_, bool>(());
                                    match result {
                                        Ok(true) => {
                                            set_plugin_status(
                                                &info.name,
                                                PluginStatus {
                                                    version: info.version.clone(),
                                                    startup_timestamp: chrono::Local::now()
                                                        .timestamp(),
                                                    plugin_path: plugin_dir
                                                        .to_str()
                                                        .unwrap()
                                                        .to_string(),
                                                },
                                            );

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
                                            "Plugin rejected init, read plugin docs to get more details"
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
                            log::error!("Error Detail: {_e}")
                        }
                    }
                }
            }
        }

        lua.globals().set("manager", manager).unwrap();

        Ok(())
    }

    pub fn on_build_start(crate_config: &CrateConfig, platform: Platform) -> anyhow::Result<()> {
        //* */
        let lua = LUA.lock().unwrap();

        if !lua.globals().contains_key("manager")? {
            return Ok(());
        }
        let manager = lua.globals().get::<_, Table>("manager")?;

        let args = lua.create_table()?;
        args.set("name", crate_config.dioxus_config.application.name.clone())?;
        args.set("platform", platform)?;
        args.set("out_dir", crate_config.out_dir.to_str().unwrap())?;
        args.set("asset_dir", crate_config.asset_dir.to_str().unwrap())?;

        for i in 1..(manager.len()? as i32 + 1) {
            let info = manager.get::<i32, PluginInfo>(i)?;
            lua.globals()
                .set("_temp_plugin_dir", info.inner.plugin_dir.clone())?;
            if let Some(func) = info.build.on_start {
                func.call::<Table, ()>(args.clone())?;
            }
        }

        Ok(())
    }

    pub fn on_build_finish(crate_config: &CrateConfig, platform: Platform) -> anyhow::Result<()> {
        //* */
        let lua = LUA.lock().unwrap();

        if !lua.globals().contains_key("manager")? {
            return Ok(());
        }
        let manager = lua.globals().get::<_, Table>("manager")?;

        let args = lua.create_table()?;
        args.set("name", crate_config.dioxus_config.application.name.clone())?;
        args.set("platform", platform)?;
        args.set("out_dir", crate_config.out_dir.to_str().unwrap())?;
        args.set("asset_dir", crate_config.asset_dir.to_str().unwrap())?;

        for i in 1..(manager.len()? as i32 + 1) {
            let info = manager.get::<i32, PluginInfo>(i)?;
            lua.globals()
                .set("_temp_plugin_dir", info.inner.plugin_dir.clone())?;
            if let Some(func) = info.build.on_finish {
                func.call::<Table, ()>(args.clone())?;
            }
        }

        Ok(())
    }

    pub fn before_serve_rebuild(
        timestamp: i64,
        files: Vec<impl AsRef<Path>>,
    ) -> anyhow::Result<()> {
        //* */
        let lua = LUA.lock().expect("Lua runtime load failed.");

        let manager = lua.globals().get::<_, Table>("manager")?;

        let args = lua.create_table()?;
        args.set("timestamp", timestamp)?;
        let files: Vec<String> = files
            .iter()
            .map(|v| v.as_ref().to_str().unwrap().to_string())
            .collect();
        args.set("changed_files", files)?;

        for i in 1..(manager.len()? as i32 + 1) {
            let info = manager.get::<i32, PluginInfo>(i)?;
            lua.globals()
                .set("_temp_plugin_dir", info.inner.plugin_dir.clone())?;
            if let Some(func) = info.serve.on_rebuild_start {
                func.call::<Table, ()>(args.clone())?;
            }
        }

        Ok(())
    }

    pub fn on_serve_start(crate_config: &CrateConfig) -> anyhow::Result<()> {
        //* */
        let lua = LUA.lock().unwrap();

        if !lua.globals().contains_key("manager")? {
            return Ok(());
        }
        let manager = lua.globals().get::<_, Table>("manager")?;

        let args = lua.create_table()?;
        args.set("name", crate_config.dioxus_config.application.name.clone())?;

        for i in 1..(manager.len()? as i32 + 1) {
            let info = manager.get::<i32, PluginInfo>(i)?;
            lua.globals()
                .set("_temp_plugin_dir", info.inner.plugin_dir.clone())?;
            if let Some(func) = info.serve.on_start {
                func.call::<Table, ()>(args.clone())?;
            }
        }

        Ok(())
    }

    pub fn on_serve_rebuild(timestamp: i64, files: Vec<PathBuf>) -> anyhow::Result<()> {
        //* */
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
            lua.globals()
                .set("_temp_plugin_dir", info.inner.plugin_dir.clone())?;
            if let Some(func) = info.serve.on_rebuild_end {
                func.call::<Table, ()>(args.clone())?;
            }
        }

        Ok(())
    }

    pub fn on_serve_shutdown(crate_config: &CrateConfig) -> anyhow::Result<()> {
        let lua = LUA.lock().expect("Lua runtime load failed.");

        if !lua.globals().contains_key("manager")? {
            return Ok(());
        }
        let manager = lua.globals().get::<_, Table>("manager")?;

        let args = lua.create_table()?;
        args.set("name", crate_config.dioxus_config.application.name.clone())?;

        for i in 1..(manager.len()? as i32 + 1) {
            let info = manager.get::<i32, PluginInfo>(i)?;
            lua.globals()
                .set("_temp_plugin_dir", info.inner.plugin_dir.clone())?;
            if let Some(func) = info.serve.on_shutdown {
                func.call::<Table, ()>(args.clone())?;
            }
        }

        Ok(())
    }

    pub fn init_plugin_dir() -> PathBuf {
        // *Done
        let app_path = app_path();
        let plugin_path = app_path.join("plugins");
        if !plugin_path.is_dir() {
            std::fs::create_dir_all(&plugin_path).expect("Create plugin directory failed.");
            let mut plugin_lock_file = std::fs::File::create(plugin_path.join("Plugin.lock"))
                .expect("Plugin file init failed.");
            let content = "{}".as_bytes();
            plugin_lock_file
                .write_all(content)
                .expect("Plugin file init failed.");
        }
        let core_path = plugin_path.join("core");
        if !core_path.is_dir() {
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

    pub fn upgrade_core_library(version: &str) -> anyhow::Result<()> {
        let plugin_path: PathBuf = crate_root().unwrap().join(".dioxus").join("plugins");
        if !plugin_path.is_dir() {
            return Err(anyhow::anyhow!("Plugin directory not found"));
        }

        let url = format!(
            "https://api.github.com/repos/DioxusLabs/cli-plugin-library/branches/{version}"
        );

        let client = reqwest::blocking::Client::new();
        let result = client
            .get(url)
            .header(reqwest::header::USER_AGENT, "mrxiaozhuox")
            .send()?
            .status();
        if !result.is_success() {
            return Err(anyhow::anyhow!("Plugin library version not found"));
        }

        // Fetch & sync from remote repo
        let mut cmd = Command::new("git");
        let cmd = cmd.current_dir(plugin_path.join("core"));
        let _res = cmd.arg("fetch").output()?;

        // Switch to new version branch
        let mut cmd = Command::new("git");
        let cmd = cmd.current_dir(plugin_path.join("core"));
        let _res = cmd.arg("switch").arg(version).output()?;

        Ok(())
    }

    pub fn remote_install_plugin(url: String) -> anyhow::Result<()> {
        let plugin_dir = Self::get_plugin_dir();
        if plugin_dir.is_none() {
            return Err(anyhow::anyhow!("Plugin system not available"));
        }
        let plugin_dir = plugin_dir.unwrap();

        let binding = url.split('/').collect::<Vec<&str>>();
        let repo_name = binding.last().unwrap();

        let target_path = plugin_dir.join(repo_name);

        if target_path.is_dir() {
            return Err(anyhow::anyhow!("Plugin directory exist."));
        }

        clone_repo(&target_path, &url)?;
        Ok(())
    }

    pub fn create_dev_plugin(vscode: bool) -> anyhow::Result<()> {
        let plugin_dir = Self::get_plugin_dir();
        if plugin_dir.is_none() {
            return Err(anyhow::anyhow!("Plugin system not available"));
        }
        let plugin_dir = plugin_dir.unwrap();

        let repo_name = "hello-dioxus-plugin";
        let target_path = plugin_dir.join(repo_name);

        if target_path.is_dir() {
            return Err(anyhow::anyhow!("Plugin directory exist."));
        }

        clone_repo(
            &target_path,
            "https://github.com/mrxiaozhuox/hello-dioxus-plugin",
        )?;

        if vscode {
            let config = serde_json::json!(
                {
                    "Lua.workspace.library": [
                        "../core/"
                    ],
                    "Lua.diagnostics.globals": [
                        "library_dir"
                    ]
                }
            );
            std::fs::create_dir(target_path.join(".vscode"))?;
            let mut file = std::fs::File::create("settings.json")?;
            file.write_all(serde_json::to_string(&config)?.as_bytes())?;
        }

        Ok(())
    }
}
