use manganis_core::BundledAsset;
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

pub(crate) use css::discover_css_references;
pub(crate) use file::is_stylesheet_asset;
pub(crate) use hash::add_hash_to_asset;

pub(crate) struct AssetProcessor<'a> {
    manifest: &'a AssetManifest,
    esbuild_path: Option<PathBuf>,
    public_asset_root: String,
}

impl<'a> AssetProcessor<'a> {
    pub(crate) fn new(
        manifest: &'a AssetManifest,
        esbuild_path: Option<PathBuf>,
        public_asset_root: impl Into<String>,
    ) -> Self {
        let mut public_asset_root = public_asset_root.into();
        if !public_asset_root.starts_with('/') {
            public_asset_root.insert(0, '/');
        }
        while public_asset_root.ends_with('/') && public_asset_root.len() > 1 {
            public_asset_root.pop();
        }
        Self {
            manifest,
            esbuild_path,
            public_asset_root,
        }
    }
}

/// A manifest of all assets collected from dependencies
///
/// This will be filled in primarily by incremental compilation artifacts.
#[derive(Debug, PartialEq, Default, Clone, Serialize, Deserialize)]
pub(crate) struct AssetManifest {
    /// Map of bundled asset name to the asset itself
    assets: BTreeMap<PathBuf, HashSet<BundledAsset>>,
}

impl AssetManifest {
    fn normalize_asset_path(path: &Path) -> PathBuf {
        dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
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
            .entry(Self::normalize_asset_path(asset_path))
            .or_default()
            .insert(bundled_asset);

        Ok(bundled_asset)
    }

    /// Insert an existing bundled asset to the manifest
    pub fn insert_asset(&mut self, asset: BundledAsset) {
        let asset_path = Self::normalize_asset_path(Path::new(asset.absolute_source_path()));
        self.assets
            .entry(asset_path)
            .or_default()
            .insert(asset);
    }

    /// Get any assets that are tied to a specific source file
    pub fn get_assets_for_source(&self, path: &Path) -> Option<&HashSet<BundledAsset>> {
        self.assets.get(&Self::normalize_asset_path(path))
    }

    /// Get the first asset that matches the given source path
    pub fn get_first_asset_for_source(&self, path: &Path) -> Option<&BundledAsset> {
        self.assets
            .get(&Self::normalize_asset_path(path))
            .and_then(|assets| assets.iter().next())
    }

    /// Check if the manifest contains a specific asset
    pub fn contains(&self, asset: &BundledAsset) -> bool {
        self.assets
            .get(&Self::normalize_asset_path(Path::new(asset.absolute_source_path())))
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

    /// Return all CSS source paths currently in this manifest.
    fn css_source_paths(&self) -> Vec<PathBuf> {
        self.assets
            .keys()
            .filter(|path| is_stylesheet_asset(path))
            .cloned()
            .collect()
    }
}
