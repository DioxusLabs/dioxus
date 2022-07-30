use mlua::UserData;

use crate::tools::app_path;

pub struct PluginDirs;
impl UserData for PluginDirs {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("plugin_dir", |_, ()| {
            let path = app_path().join("plugins");
            Ok(path.to_str().unwrap().to_string())
        });
        methods.add_function("self_dir", |_, name: String| {
            let path = app_path().join("plugins").join(name);
            Ok(path.to_str().unwrap().to_string())
        });
        methods.add_function("document_dir", |_, ()| {
            let path = dirs::document_dir().unwrap().to_str().unwrap().to_string();
            Ok(path)
        });
        methods.add_function("download_dir", |_, ()| {
            let path = dirs::download_dir().unwrap().to_str().unwrap().to_string();
            Ok(path)
        });
        methods.add_function("cache_dir", |_, ()| {
            let path = dirs::cache_dir().unwrap().to_str().unwrap().to_string();
            Ok(path)
        });
    }
}