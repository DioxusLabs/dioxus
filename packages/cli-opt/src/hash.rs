//! Utilities for creating hashed paths to assets in Manganis. This module defines [`AssetHash`] which is used to create a hashed path to an asset in both the CLI and the macro.

use std::{
    hash::{Hash, Hasher},
    io::Read,
    path::{Path, PathBuf},
};

use crate::{
    css::hash_scss,
    file::{resolve_asset_options, ResolvedAssetType},
    js::hash_js,
};
use manganis::{AssetOptions, BundledAsset};

/// The opaque hash type manganis uses to identify assets. Each time an asset or asset options change, this hash will
/// change. This hash is included in the URL of the bundled asset for cache busting.
struct AssetHash {
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
        file_path: impl AsRef<Path>,
    ) -> anyhow::Result<AssetHash> {
        hash_file(options, file_path.as_ref())
    }
}

/// Process a specific file asset with the given options reading from the source and writing to the output path
fn hash_file(options: &AssetOptions, source: &Path) -> anyhow::Result<AssetHash> {
    // Create a hasher
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    options.hash(&mut hash);

    // Hash the version of CLI opt
    hash.write(crate::build_info::version().as_bytes());
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
    let resolved_options = resolve_asset_options(source, options.variant());

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
        ResolvedAssetType::CssModule(_)
        | ResolvedAssetType::Css(_)
        | ResolvedAssetType::Image(_)
        | ResolvedAssetType::Json
        | ResolvedAssetType::File => {
            hash_file_contents(source, hasher)?;
        }

        // Or the folder contents recursively
        ResolvedAssetType::Folder(_) => {
            for file in std::fs::read_dir(source)?.flatten() {
                let path = file.path();
                hash_file_with_options(
                    // We can't reuse the options here since they contain the source variant which no
                    // longer applies to the nested files
                    //
                    // We don't hash nested files either since we assume the parent here is already being hashed
                    // (or being opted out of hashing)
                    &AssetOptions::builder()
                        .with_hash_suffix(false)
                        .into_asset_options(),
                    &path,
                    hasher,
                    true,
                )?;
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

/// Add a hash to the asset, or log an error if it fails
pub fn add_hash_to_asset(asset: &mut BundledAsset) {
    let source = asset.absolute_source_path();
    match AssetHash::hash_file_contents(asset.options(), source) {
        Ok(hash) => {
            let options = *asset.options();

            // Set the bundled path to the source path with the hash appended before the extension
            let source_path = PathBuf::from(source);
            let Some(file_name) = source_path.file_name() else {
                tracing::error!("Failed to get file name from path: {source}");
                return;
            };

            // The output extension path is the extension set by the options
            // or the extension of the source file if we don't recognize the file
            let mut ext = asset.options().extension().map(Into::into).or_else(|| {
                source_path
                    .extension()
                    .map(|ext| ext.to_string_lossy().to_string())
            });

            // Rewrite scss as css
            if let Some("scss" | "sass") = ext.as_deref() {
                ext = Some("css".to_string());
            }

            let hash = hash.bytes();
            let hash = hash
                .iter()
                .map(|byte| format!("{byte:x}"))
                .collect::<String>();
            let file_stem = source_path.file_stem().unwrap_or(file_name);
            let mut bundled_path = if asset.options().hash_suffix() {
                PathBuf::from(format!("{}-dxh{hash}", file_stem.to_string_lossy()))
            } else {
                PathBuf::from(file_stem)
            };

            if let Some(ext) = ext {
                // Push the extension to the bundled path. There may be multiple extensions (e.g. .js.map)
                // with one left after the file_stem is extracted above so we need to push the extension
                // instead of setting it
                bundled_path.as_mut_os_string().push(format!(".{ext}"));
            }

            let bundled_path = bundled_path.to_string_lossy().to_string();

            *asset = BundledAsset::new(source, &bundled_path, options);
        }
        Err(err) => {
            tracing::error!("Failed to hash asset {source}: {err}");
        }
    }
}
