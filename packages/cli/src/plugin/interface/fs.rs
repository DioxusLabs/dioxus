use std::{
    fs::{create_dir, create_dir_all, remove_dir_all, File},
    io::{Read, Write},
    path::PathBuf,
};

use crate::tools::extract_zip;
use flate2::read::GzDecoder;
use mlua::UserData;
use tar::Archive;

pub struct PluginFileSystem;
impl UserData for PluginFileSystem {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
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

            let file = std::fs::File::create(path);
            if file.is_err() {
                return Ok(false);
            }

            if file.unwrap().write_all(content.as_bytes()).is_err() {
                return Ok(false);
            }

            Ok(true)
        });
        methods.add_function("move_file", |_, args: (String, String)| {
            let file = PathBuf::from(args.0);
            let target = PathBuf::from(args.1);
            if !file.is_file() {
                return Ok(false);
            }
            let options = fs_extra::file::CopyOptions::new();
            let res = fs_extra::file::move_file(file, target, &options);
            if res.is_err() {
                return Ok(false);
            }
            Ok(true)
        });
        methods.add_function("copy_file", |_, args: (String, String)| {
            let file = PathBuf::from(args.0);
            let target = PathBuf::from(args.1);
            if !file.is_file() {
                return Ok(false);
            }
            let options = fs_extra::file::CopyOptions::new();
            let res = fs_extra::file::copy(file, target, &options);
            Ok(res.is_ok())
        });
        methods.add_function("read_dir", |_, path: String| {
            let mut result = Vec::new();
            let path = PathBuf::from(path);
            if path.is_dir() {
                let list = std::fs::read_dir(path).unwrap();
                for i in list.flatten() {
                    result.push(i.path().display().to_string());
                }
            }
            Ok(result)
        });
        methods.add_function("unzip_file", |_, args: (String, String)| {
            let file = PathBuf::from(args.0);
            let target = PathBuf::from(args.1);
            let res = extract_zip(&file, &target);
            if res.is_err() {
                return Ok(false);
            }
            Ok(true)
        });
        methods.add_function("untar_gz_file", |_, args: (String, String)| {
            let file = PathBuf::from(args.0);
            let target = PathBuf::from(args.1);

            let tar_gz = if let Ok(v) = File::open(file) {
                v
            } else {
                return Ok(false);
            };

            let tar = GzDecoder::new(tar_gz);
            let mut archive = Archive::new(tar);
            if archive.unpack(&target).is_err() {
                return Ok(false);
            }

            Ok(true)
        });
    }
}
