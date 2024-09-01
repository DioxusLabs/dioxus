use crate::builder::{
    BuildMessage, BuildRequest, MessageSource, MessageType, Stage, UpdateBuildProgress, UpdateStage,
};
use crate::Result;
use anyhow::Context;
use brotli::enc::BrotliEncoderParams;
use futures_channel::mpsc::UnboundedSender;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::{ffi::OsString, path::PathBuf};
use std::{fs::File, io::Write};
use tracing::Level;
use walkdir::WalkDir;

mod file;
mod manifest;

// pub use file::process_file;
// pub use folder::process_folder;
pub use manifest::*;

pub(crate) fn copy_dir_to(
    src_dir: PathBuf,
    dest_dir: PathBuf,
    pre_compress: bool,
) -> std::io::Result<()> {
    let entries = std::fs::read_dir(&src_dir)?;
    let mut children: Vec<std::thread::JoinHandle<std::io::Result<()>>> = Vec::new();

    for entry in entries.flatten() {
        let entry_path = entry.path();
        let path_relative_to_src = entry_path.strip_prefix(&src_dir).unwrap();
        let output_file_location = dest_dir.join(path_relative_to_src);
        children.push(std::thread::spawn(move || {
            if entry.file_type()?.is_dir() {
                // If the file is a directory, recursively copy it into the output directory
                if let Err(err) =
                    copy_dir_to(entry_path.clone(), output_file_location, pre_compress)
                {
                    tracing::error!(
                        "Failed to pre-compress directory {}: {}",
                        entry_path.display(),
                        err
                    );
                }
            } else {
                // Make sure the directory exists
                std::fs::create_dir_all(output_file_location.parent().unwrap())?;
                // Copy the file to the output directory
                std::fs::copy(&entry_path, &output_file_location)?;

                // Then pre-compress the file if needed
                if pre_compress {
                    if let Err(err) = pre_compress_file(&output_file_location) {
                        tracing::error!(
                            "Failed to pre-compress static assets {}: {}",
                            output_file_location.display(),
                            err
                        );
                    }
                    // If pre-compression isn't enabled, we should remove the old compressed file if it exists
                } else if let Some(compressed_path) = compressed_path(&output_file_location) {
                    _ = std::fs::remove_file(compressed_path);
                }
            }

            Ok(())
        }));
    }
    for child in children {
        child.join().unwrap()?;
    }
    Ok(())
}

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
