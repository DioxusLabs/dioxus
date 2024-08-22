use std::{
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
};

use anyhow::Context;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::{config, FileOptions};

// mod file;
// mod folder;
mod error;
mod file;
mod meta;
mod resource;
mod tailwind;

// pub use folder::*;
pub use error::*;
pub use file::*;
pub use meta::*;
pub use resource::*;
pub use tailwind::*;

/// The maximum length of a path segment
const MAX_PATH_LENGTH: usize = 128;

/// The length of the hash in the output path
const HASH_SIZE: usize = 16;

/// The type of asset
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum AssetType {
    /// A resource asset in the form of a URI
    ///
    /// Typically a file, but could be a folder or a remote URL
    Resource(ResourceAsset),

    /// A tailwind class asset
    Tailwind(TailwindAsset),

    /// A metadata asset
    Metadata(MetadataAsset),
}
