use std::{io::Cursor, path::PathBuf};

use mlua::UserData;

pub struct PluginNetwork;
impl UserData for PluginNetwork {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("download_file", |_, args: (String, String)| {
            let url = args.0;
            let path = args.1;

            let resp = reqwest::blocking::get(url);
            if let Ok(resp) = resp {
                let mut content = Cursor::new(resp.bytes().unwrap());
                let file = std::fs::File::create(PathBuf::from(path));
                if file.is_err() {
                    return Ok(false);
                }
                let mut file = file.unwrap();
                let res = std::io::copy(&mut content, &mut file);
                return Ok(res.is_ok());
            }

            Ok(false)
        });
    }
}
