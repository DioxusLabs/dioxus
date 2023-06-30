use std::path::PathBuf;

use mlua::{UserData, Variadic};

pub struct PluginPath;
impl UserData for PluginPath {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        // join function
        methods.add_function("join", |_, args: Variadic<String>| {
            let mut path = PathBuf::new();
            for i in args {
                path = path.join(i);
            }
            Ok(path.to_str().unwrap().to_string())
        });

        // parent function
        methods.add_function("parent", |_, path: String| {
            let current_path = PathBuf::from(&path);
            let parent = current_path.parent();

            if let Some(parent) = parent {
                Ok(parent.to_str().unwrap().to_string())
            } else {
                Ok(path)
            }
        });
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
