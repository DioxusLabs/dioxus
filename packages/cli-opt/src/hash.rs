//! Utilities for creating hashed paths to assets in Manganis. This module defines [`AssetHash`] which is used to create a hashed path to an asset in both the CLI and the macro.

use std::{hash::Hasher, io::Read, path::Path};

use crate::{
    css::hash_scss,
    file::{resolve_asset_options, ResolvedAssetType},
    js::hash_js,
};
use manganis::AssetOptions;

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
    pub fn hash_file_contents(
        options: &AssetOptions,
        file_path: &Path,
    ) -> anyhow::Result<AssetHash> {
        hash_file(options, file_path)
    }
}

/// Process a specific file asset with the given options reading from the source and writing to the output path
fn hash_file(options: &AssetOptions, source: &Path) -> anyhow::Result<AssetHash> {
    // Create a hasher
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    hash_file_with_options(options, source, &mut hash, false)?;

    let hash = hash.finish();
    Ok(AssetHash::new(hash))
}

/// Process a specific file asset with additional options
pub(crate) fn hash_file_with_options(
    options: &AssetOptions,
    source: &Path,
    hasher: &mut impl Hasher,
    in_folder: bool,
) -> anyhow::Result<()> {
    let resolved_options = resolve_asset_options(source, options);

    match &resolved_options {
        // Scss and JS can import files during the bundling process. We need to hash
        // both the files themselves and any imports they have
        ResolvedAssetType::Scss(options) => {
            hash_scss(options, source, hasher)?;
        }
        ResolvedAssetType::Js(options) => {
            hash_js(options, source, hasher, !in_folder)?;
        }

        // Otherwise, we can just hash the file contents
        ResolvedAssetType::Css(_)
        | ResolvedAssetType::Image(_)
        | ResolvedAssetType::Json
        | ResolvedAssetType::File => {
            hash_file_contents(source, hasher)?;
        }
        // Or the folder contents recursively
        ResolvedAssetType::Folder(_) => {
            let files = std::fs::read_dir(source)?;
            for file in files.flatten() {
                let path = file.path();
                hash_file_with_options(&options, &path, hasher, true)?;
            }
        }
    }

    Ok(())
}

pub(crate) fn hash_file_contents(source: &Path, hasher: &mut impl Hasher) -> anyhow::Result<()> {
    // Otherwise, open the file to get its contents
    let mut file = std::fs::File::open(source)?;

    // We add a hash to the end of the file so it is invalidated when the bundled version of the file changes
    // The hash includes the file contents, the options, and the version of manganis. From the macro, we just
    // know the file contents, so we only include that hash
    let mut buffer = [0; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.write(&buffer[..read]);
    }
    Ok(())
}
