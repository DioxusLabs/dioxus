//! Utilities for working with cargo and rust files
use std::error::Error;
use std::{
    env,
    fmt::{Display, Formatter},
    fs,
    path::{Path, PathBuf},
    process::Command,
    str,
};

#[derive(Debug, Clone)]
pub struct CargoError {
    msg: String,
}

impl CargoError {
    pub fn new(msg: String) -> Self {
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

/// Some fields parsed from `cargo metadata` command
pub struct Metadata {
    pub workspace_root: PathBuf,
    pub target_directory: PathBuf,
}

/// Returns the root of the crate that the command is run from
///
/// If the command is run from the workspace root, this will return the top-level Cargo.toml
pub fn crate_root() -> Result<PathBuf, CargoError> {
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

impl Metadata {
    /// Returns the struct filled from `cargo metadata` output
    /// TODO @Jon, find a different way that doesn't rely on the cargo metadata command (it's slow)
    pub fn get() -> Result<Self, CargoError> {
        let output = Command::new("cargo")
            .args(["metadata"])
            .output()
            .map_err(|_| CargoError::new("Manifset".to_string()))?;

        if !output.status.success() {
            let mut msg = str::from_utf8(&output.stderr).unwrap().trim();
            if msg.starts_with("error: ") {
                msg = &msg[7..];
            }

            return Err(CargoError::new(msg.to_string()));
        }

        let stdout = str::from_utf8(&output.stdout).unwrap();
        if let Some(line) = stdout.lines().next() {
            let meta: serde_json::Value = serde_json::from_str(line)
                .map_err(|_| CargoError::new("InvalidOutput".to_string()))?;

            let workspace_root = meta["workspace_root"]
                .as_str()
                .ok_or_else(|| CargoError::new("InvalidOutput".to_string()))?
                .into();

            let target_directory = meta["target_directory"]
                .as_str()
                .ok_or_else(|| CargoError::new("InvalidOutput".to_string()))?
                .into();

            return Ok(Self {
                workspace_root,
                target_directory,
            });
        }

        Err(CargoError::new("InvalidOutput".to_string()))
    }
}
