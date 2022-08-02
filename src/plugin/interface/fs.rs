use std::{
    fs::{create_dir, create_dir_all, remove_dir_all},
    path::PathBuf, io::{Read, Write},
};

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
        methods.add_function("create_dir", |_, args: (String, bool)| {
            let path = args.0;
            let recursive = args.1;
            let path = PathBuf::from(path);
            if !path.exists() {
                let v = if recursive {
                    create_dir_all(path)
                } else {
                    create_dir(path)
                };
                return Ok(v.is_ok());
            }
            Ok(true)
        });
        methods.add_function("remove_dir", |_, path: String| {
            let path = PathBuf::from(path);
            let r = remove_dir_all(path);
            Ok(r.is_ok())
        });
        methods.add_function("file_get_content", |_, path: String| {
            let path = PathBuf::from(path);
            let mut file = std::fs::File::open(path)?;
            let mut buffer = String::new();
            file.read_to_string(&mut buffer)?;
            Ok(buffer)
        });
        methods.add_function("file_set_content", |_, args: (String, String)| {
            let path = args.0;
            let content = args.1;
            let path = PathBuf::from(path);
            let mut file = std::fs::File::create(path)?;
            file.write_all(content.as_bytes())?;
            Ok(())
        });
    }
}
