use crate::{Result, Error};
use std::{path::PathBuf, fs, ffi::OsStr};

/// Represents a File on the device's storage system.
pub struct File {
    /// The name of the file.
    pub name: String,
    /// The path to the file.
    pub path: PathBuf,
    /// The file's type.
    pub file_type: FileType,
}

/// Represents a file's type.
pub enum FileType {
    Html,

    // Styling
    Css,
    Scss,
    Sass,

    // Programming
    JavaScript,
    Rust,
    Wasm,

    // Images
    Png,
    Jpg,
    Jpeg,
    Webp,
    Apng,
    Avif,
    Gif,
    Svg,

    // Misc file, just copy to dest
    Misc(String),
}

impl From<String> for FileType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "html" => Self::Html,

            // Styling
            "css" => Self::Css,
            "scss" => Self::Scss,
            "sass" => Self::Sass,

            // Programming
            "js" => Self::JavaScript,
            "rs" => Self::Rust,
            "wasm" => Self::Wasm,

            // Images
            "png" => Self::Png,
            "jpg" => Self::Jpg,
            "jpeg" => Self::Jpeg,
            "webp" => Self::Webp,
            "apng" => Self::Apng,
            "avif" => Self::Avif,
            "gif" => Self::Gif,
            "svg" => Self::Svg,
            v => Self::Misc(v.to_string()),
        }
    }
}

pub fn from_dir(dir_path: PathBuf) -> Result<Vec<File>> {
    let mut files = Vec::new();

    // recursively get files
    if dir_path.is_dir() {
        for item in fs::read_dir(dir_path)? {
            let item = item?;
            let path = item.path();

            if path.is_dir() {
                // If directory, get files from it
                from_dir(path)?;
            } else {
                let file_name = path.file_stem().and_then(OsStr::to_str);
                let file_name = match file_name {
                    Some(v) => v,
                    None => return Err(Error::ParseError("Failed to determine file name".to_string())),
                };

                let extension = path.extension().and_then(OsStr::to_str);
                let extension = match extension {
                    Some(v) => v,
                    None => return Err(Error::ParseError("Failed to determine file extension".to_string())),
                };

                let file_type = FileType::from(extension.to_string());

                // Add to list of files
                files.push(File {
                    name: file_name.to_string(),
                    path,
                    file_type,
                })
            }
        }
    }

    Ok(files)
}
