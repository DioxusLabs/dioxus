use std::path::PathBuf;

use mlua::UserData;

pub struct PluginPath;
impl UserData for PluginPath {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("join", |_, args: (String, String)| {
            let current_path = PathBuf::from(args.0);
            let new_path = current_path.join(args.1);
            Ok(new_path.to_str().unwrap().to_string())
        });
    }
}