//! Utilities for working with cargo and rust files
use std::error::Error;
use std::{
    env,
    ffi::OsStr,
    fmt::{Display, Formatter},
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub(crate) struct CargoError {
    msg: String,
}

impl CargoError {
    pub(crate) fn new(msg: String) -> Self {
        Self { msg }
    }
}

impl Display for CargoError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "CargoError: {}", self.msg)
    }
}

impl Error for CargoError {}

/// How many parent folders are searched for a `Cargo.toml`
const MAX_ANCESTORS: u32 = 10;

/// Returns the root of the crate that the command is run from
///
/// If the command is run from the workspace root, this will return the top-level Cargo.toml
pub(crate) fn crate_root() -> Result<PathBuf, CargoError> {
    // From the current directory we work our way up, looking for `Cargo.toml`
    env::current_dir()
        .ok()
        .and_then(|mut wd| {
            for _ in 0..MAX_ANCESTORS {
                if contains_manifest(&wd) {
                    return Some(wd);
                }
                if !wd.pop() {
                    break;
                }
            }
            None
        })
        .ok_or_else(|| {
            CargoError::new("Failed to find directory containing Cargo.toml".to_string())
        })
}

/// Checks if the directory contains `Cargo.toml`
fn contains_manifest(path: &Path) -> bool {
    fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .any(|ent| &ent.file_name() == "Cargo.toml")
        })
        .unwrap_or(false)
}

/// Collects all `.rs` files in the provided directory, respecting files to ignore (e.g. `.gitignore`)
pub(crate) fn collect_rs_files(dir: impl AsRef<Path>) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for result in ignore::Walk::new(dir) {
        let path = result.unwrap().into_path();
        if let Some(ext) = path.extension() {
            if ext == OsStr::new("rs") {
                files.push(path);
            }
        }
    }
    files
}
