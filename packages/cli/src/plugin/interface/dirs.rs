use mlua::UserData;

use crate::tools::app_path;

pub struct PluginDirs;
impl UserData for PluginDirs {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("plugins_dir", |_, ()| {
            let path = app_path().join("plugins");
            Ok(path.to_str().unwrap().to_string())
        });
    }
}
