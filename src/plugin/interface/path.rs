use std::path::PathBuf;

use mlua::UserData;

pub struct PluginPath;
impl UserData for PluginPath {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        // join function
        methods.add_function("join", |_, args: (String, String)| {
            let current_path = PathBuf::from(args.0);
            let new_path = current_path.join(args.1);
            Ok(new_path.to_str().unwrap().to_string())
        });

        // parent function
        methods.add_function("parent", |_, path: String| {
            let current_path = PathBuf::from(&path);
            let parent = current_path.parent();
            if parent.is_none() {
                return Ok(path);
            } else {
                return Ok(parent.unwrap().to_str().unwrap().to_string());
            }
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
