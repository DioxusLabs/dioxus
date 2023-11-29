use std::{
    collections::HashMap,
    io::{Read, Write},
};

use crate::crate_root;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStatus {
    pub version: String,
    pub startup_timestamp: i64,
    pub plugin_path: String,
}

pub fn plugins_status() -> HashMap<String, PluginStatus> {
    let plugin_path = crate_root().unwrap().join(".dioxus").join("plugins");

    if let Ok(mut lock_file) = std::fs::File::open(plugin_path.join("Plugin.lock")) {
        let mut lock_content = String::new();
        if lock_file.read_to_string(&mut lock_content).is_err() {
            return HashMap::new();
        }
        serde_json::from_str::<HashMap<String, PluginStatus>>(&lock_content)
            .ok()
            .unwrap_or_default()
    } else {
        let mut lock_file = std::fs::File::create(plugin_path.join("Plugin.lock")).unwrap();
        let empty_lock = String::from("{}");
        lock_file
            .write_all(empty_lock.as_bytes())
            .unwrap_or_default();

        HashMap::new()
    }
}

pub fn get_plugin_status(name: &str) -> Option<PluginStatus> {
    let v = plugins_status();
    let content = v.get(name)?;
    Some(content.clone())
}

pub fn set_plugin_status(name: &str, info: PluginStatus) {
    let mut plugins = plugins_status();
    plugins.insert(name.to_string(), info);

    let plugin_path = crate_root().unwrap().join(".dioxus").join("plugins");
    let mut lock_file = std::fs::File::create(plugin_path.join("Plugin.lock")).unwrap();

    let content = serde_json::to_string(&plugins).unwrap();
    lock_file.write_all(content.as_bytes()).unwrap();
}
