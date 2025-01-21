//! Methods for working with the filesystem that are faster than the std fs methods
//! Uses stuff like rayon, caching, and other optimizations
//!
//! Allows configuration in case you want to do some work while copying and allows you to track progress

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use brotli::enc::BrotliEncoderParams;
use walkdir::WalkDir;

/// Get the path to the compressed version of a file
fn compressed_path(path: &Path) -> Option<PathBuf> {
    let new_extension = match path.extension() {
        Some(ext) => {
            if ext.to_string_lossy().to_lowercase().ends_with("br") {
                return None;
            }
            let mut ext = ext.to_os_string();
            ext.push(".br");
            ext
        }
        None => OsString::from("br"),
    };

    Some(path.with_extension(new_extension))
}

/// pre-compress a file with brotli
pub(crate) fn pre_compress_file(path: &Path) -> std::io::Result<()> {
    let Some(compressed_path) = compressed_path(path) else {
        return Ok(());
    };

    let file = std::fs::File::open(path)?;
    let mut stream = std::io::BufReader::new(file);
    let mut buffer = std::fs::File::create(compressed_path)?;
    let params = BrotliEncoderParams::default();
    brotli::BrotliCompress(&mut stream, &mut buffer, &params)?;

    Ok(())
}

/// pre-compress all files in a folder
pub(crate) fn pre_compress_folder(path: &Path, pre_compress: bool) -> std::io::Result<()> {
    let walk_dir = WalkDir::new(path);
    for entry in walk_dir.into_iter().filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        if entry_path.is_file() {
            if pre_compress {
                if let Err(err) = pre_compress_file(entry_path) {
                    tracing::error!("Failed to pre-compress file {entry_path:?}: {err}");
                }
            }
            // If pre-compression isn't enabled, we should remove the old compressed file if it exists
            else if let Some(compressed_path) = compressed_path(entry_path) {
                _ = std::fs::remove_file(compressed_path);
            }
        }
    }
    Ok(())
}
