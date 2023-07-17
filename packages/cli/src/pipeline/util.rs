use crate::{Result, Error};
use std::{path::PathBuf, fs};

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
                let file_name = item.file_name();

                // Split between actual name and extension
                let split: Vec<_> = file_name.to_str().unwrap().split(".").collect();
                if split.len() > 2 {
                    return Err(Error::ParseError("Failed to determine asset name and extension: there is more than one `.` which is not supported.".to_string()));
                }

                // We already know that these exist because of if statement above.
                let name = split.get(0).unwrap().to_string();
                let file_type = FileType::from(split.get(1).unwrap().to_string());

                // Add to list of files
                files.push(File {
                    name,
                    path,
                    file_type,
                })
            }
        }
    }

    Ok(files)
}
