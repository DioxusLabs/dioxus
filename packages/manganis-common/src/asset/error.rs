use std::{
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
};

use anyhow::Context;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::{config, FileOptions};

/// Error while checking an asset exists
#[derive(Debug)]
pub enum AssetError {
    /// The relative path does not exist
    NotFoundRelative(PathBuf, String),
    /// The path exist but is not a file
    NotFile(PathBuf),
    /// The path exist but is not a folder
    NotFolder(PathBuf),
    /// Unknown IO error
    IO(PathBuf, std::io::Error),
}

impl Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetError::NotFoundRelative(manifest_dir, path) =>
                write!(f,"cannot find file `{}` in `{}`, please make sure it exists.\nAny relative paths are resolved relative to the manifest directory.",
                       path,
                       manifest_dir.display()
                ),
            AssetError::NotFile(absolute_path) =>
                write!(f, "`{}` is not a file, please choose a valid asset.\nAny relative paths are resolved relative to the manifest directory.", absolute_path.display()),
            AssetError::NotFolder(absolute_path) =>
                write!(f, "`{}` is not a folder, please choose a valid asset.\nAny relative paths are resolved relative to the manifest directory.", absolute_path.display()),
            AssetError::IO(absolute_path, err) =>
                write!(f, "unknown error when accessing `{}`: \n{}", absolute_path.display(), err)
        }
    }
}

/// An error that can occur while collecting assets without CLI support
#[derive(Debug)]
pub enum ManganisSupportError {
    /// An error that can occur while collecting assets from other packages without CLI support
    ExternalPackageCollection,
    /// Manganis failed to find the current package's manifest
    FailedToFindCargoManifest,
}

impl Display for ManganisSupportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExternalPackageCollection => write!(f, "Attempted to collect assets from other packages without a CLI that supports Manganis. Please recompile with a CLI that supports Manganis like the `dioxus-cli`."),
            Self::FailedToFindCargoManifest => write!(f, "Manganis failed to find the current package's manifest. Please recompile with a CLI that supports Manganis like the `dioxus-cli`."),
        }
    }
}

impl std::error::Error for ManganisSupportError {}
