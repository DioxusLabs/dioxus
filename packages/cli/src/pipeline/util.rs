use crate::{Error, Result};
use std::{ffi::OsStr, fs, path::PathBuf};

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
#[derive(PartialEq, Clone)]
pub enum FileType {
    Html,

    // Styling
    Css,
    SassType,

    // Programming
    JavaScript,
    Rust,
    Wasm,
    Image,
    Unknown,
}

impl From<&PathBuf> for FileType {
    fn from(value: &PathBuf) -> Self {
        let extension = match value.extension().and_then(OsStr::to_str) {
            Some(ext) => ext.to_string(),
            None => return Self::Unknown,
        };

        FileType::from(extension)
    }
}

impl From<String> for FileType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "html" => Self::Html,
            "htm" => Self::Html,

            // Styling
            "css" => Self::Css,
            "scss" => Self::SassType,
            "sass" => Self::SassType,

            // Programming
            "js" => Self::JavaScript,
            "rs" => Self::Rust,
            "wasm" => Self::Wasm,

            // Images
            "png" => Self::Image,
            "jpg" => Self::Image,
            "jpeg" => Self::Image,
            "webp" => Self::Image,
            "apng" => Self::Image,
            "avif" => Self::Image,
            "gif" => Self::Image,
            "svg" => Self::Image,
            _ => Self::Unknown,
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
                files.append(&mut from_dir(path)?);
            } else {
                let file_name = path.file_stem().and_then(OsStr::to_str);
                let file_name = match file_name {
                    Some(v) => v,
                    None => {
                        return Err(Error::ParseError(
                            "Failed to determine file name".to_string(),
                        ))
                    }
                };

                let file_type = FileType::from(&path);

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
