use std::path::PathBuf;

use mlua::UserData;

pub struct PluginFileSystem;
impl UserData for PluginFileSystem {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("exists", |_, path: String| {
            let path = PathBuf::from(path);
            Ok(path.exists())
        });
        methods.add_function("is_dir", |_, path: String| {
            let path = PathBuf::from(path);
            Ok(path.is_dir())
        });
        methods.add_function("is_file", |_, path: String| {
            let path = PathBuf::from(path);
            Ok(path.is_file())
        });
    }
}