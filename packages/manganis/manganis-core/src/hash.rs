//! Utilities for creating hashed paths to assets in Manganis. This module defines [`AssetHash`] which is used to create a hashed path to an asset in both the CLI and the macro.

use std::{
    error::Error,
    hash::{Hash, Hasher},
    io::Read,
    path::{Path, PathBuf},
};

/// An error that can occur when hashing an asset
#[derive(Debug)]
#[non_exhaustive]
pub enum AssetHashError {
    /// An io error occurred
    IoError { err: std::io::Error, path: PathBuf },
}

impl std::fmt::Display for AssetHashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetHashError::IoError { path, err } => {
                write!(f, "Failed to read file: {}; {}", path.display(), err)
            }
        }
    }
}

impl Error for AssetHashError {}

/// The opaque hash type manganis uses to identify assets. Each time an asset or asset options change, this hash will
/// change. This hash is included in the URL of the bundled asset for cache busting.
pub struct AssetHash {
    /// We use a wrapper type here to hide the exact size of the hash so we can switch to a sha hash in a minor version bump
    hash: [u8; 8],
}

impl AssetHash {
    /// Create a new asset hash
    const fn new(hash: u64) -> Self {
        Self {
            hash: hash.to_le_bytes(),
        }
    }

    /// Get the hash bytes
    pub const fn bytes(&self) -> &[u8] {
        &self.hash
    }

    /// Create a new asset hash for a file. The input file to this function should be fully resolved
    pub fn hash_file_contents(file_path: &Path) -> Result<AssetHash, AssetHashError> {
        // Create a hasher
        let mut hash = std::collections::hash_map::DefaultHasher::new();

        // If this is a folder, hash the folder contents
        if file_path.is_dir() {
            let files = std::fs::read_dir(file_path).map_err(|err| AssetHashError::IoError {
                err,
                path: file_path.to_path_buf(),
            })?;
            for file in files.flatten() {
                let path = file.path();
                Self::hash_file_contents(&path)?.bytes().hash(&mut hash);
            }
            let hash = hash.finish();
            return Ok(AssetHash::new(hash));
        }

        // Otherwise, open the file to get its contents
        let mut file = std::fs::File::open(file_path).map_err(|err| AssetHashError::IoError {
            err,
            path: file_path.to_path_buf(),
        })?;

        // We add a hash to the end of the file so it is invalidated when the bundled version of the file changes
        // The hash includes the file contents, the options, and the version of manganis. From the macro, we just
        // know the file contents, so we only include that hash
        let mut buffer = [0; 8192];
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|err| AssetHashError::IoError {
                    err,
                    path: file_path.to_path_buf(),
                })?;
            if read == 0 {
                break;
            }
            hash.write(&buffer[..read]);
        }

        Ok(AssetHash::new(hash.finish()))
    }
}
