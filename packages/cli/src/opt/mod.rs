use manganis::SwiftPackageMetadata;
use manganis_core::{AndroidArtifactMetadata, BundledAsset};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

mod css;
mod file;
mod folder;
mod hash;
mod image;
mod js;
mod json;

pub(crate) use file::process_file_to;
pub(crate) use hash::add_hash_to_asset;

/// A manifest of all assets collected from dependencies. This is persisted to disk for users to be
/// able to pick up the result of asset extraction.
///
/// This will be filled in primarily by incremental compilation artifacts.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub(crate) struct AppManifest {
    /// Stable since 0.7.0
    pub cli_version: String,

    /// Map of bundled asset name to the asset itself
    pub assets: BTreeMap<PathBuf, HashSet<BundledAsset>>,

    pub android_artifacts: Vec<AndroidArtifactMetadata>,

    pub swift_sources: Vec<SwiftPackageMetadata>,
}

impl AppManifest {
    pub fn new() -> Self {
        Self {
            cli_version: crate::VERSION.to_string(),
            android_artifacts: Default::default(),
            swift_sources: Default::default(),
            assets: Default::default(),
        }
    }

    /// Manually add an asset to the manifest
    pub fn register_asset(
        &mut self,
        asset_path: &Path,
        options: manganis::AssetOptions,
    ) -> anyhow::Result<BundledAsset> {
        let output_path_str = asset_path.to_str().ok_or(anyhow::anyhow!(
            "Failed to convert wasm bindgen output path to string"
        ))?;

        let mut bundled_asset =
            manganis::macro_helpers::create_bundled_asset(output_path_str, options);
        add_hash_to_asset(&mut bundled_asset);

        self.assets
            .entry(asset_path.to_path_buf())
            .or_default()
            .insert(bundled_asset);

        Ok(bundled_asset)
    }

    /// Insert an existing bundled asset to the manifest
    pub fn insert_asset(&mut self, asset: BundledAsset) {
        let asset_path = asset.absolute_source_path();
        self.assets
            .entry(asset_path.into())
            .or_default()
            .insert(asset);
    }

    /// Get any assets that are tied to a specific source file
    pub fn get_assets_for_source(&self, path: &Path) -> Option<&HashSet<BundledAsset>> {
        self.assets.get(path)
    }

    /// Get the first asset that matches the given source path
    pub fn get_first_asset_for_source(&self, path: &Path) -> Option<&BundledAsset> {
        self.assets
            .get(path)
            .and_then(|assets| assets.iter().next())
    }

    /// Check if the manifest contains a specific asset
    pub fn contains(&self, asset: &BundledAsset) -> bool {
        self.assets
            .get(&PathBuf::from(asset.absolute_source_path()))
            .is_some_and(|assets| assets.contains(asset))
    }

    /// Iterate over all the assets with unique output paths in the manifest. This will not include
    /// assets that have different source paths, but the same file contents.
    pub fn unique_assets(&self) -> impl Iterator<Item = &BundledAsset> {
        let mut seen = HashSet::new();
        self.assets
            .values()
            .flat_map(|assets| assets.iter())
            .filter(move |asset| seen.insert(asset.bundled_path()))
    }
}
