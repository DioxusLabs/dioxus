use anyhow::Context;
use manganis::AssetOptions;
use manganis_core::BundledAsset;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

mod build_info;
mod css;
mod file;
mod folder;
mod hash;
mod image;
mod js;
mod json;

pub use file::process_file_to;
pub use hash::add_hash_to_asset;

/// A manifest of all assets collected from dependencies
///
/// This will be filled in primarily by incremental compilation artifacts.
#[derive(Debug, PartialEq, Default, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    /// Map of bundled asset name to the asset itself
    assets: BTreeMap<PathBuf, HashSet<BundledAsset>>,
}

impl AssetManifest {
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

    pub fn load_from_file(path: &Path) -> anyhow::Result<Self> {
        let src = std::fs::read_to_string(path)?;

        serde_json::from_str(&src)
            .with_context(|| format!("Failed to parse asset manifest from {path:?}\n{src}"))
    }
}

/// Optimize a list of assets in parallel
pub fn optimize_all_assets(
    assets_to_transfer: Vec<(PathBuf, PathBuf, AssetOptions)>,
    on_optimization_start: impl FnMut(&Path, &Path, &AssetOptions) + Sync + Send,
    on_optimization_end: impl FnMut(&Path, &Path, &AssetOptions) + Sync + Send,
) -> anyhow::Result<()> {
    let on_optimization_start = Arc::new(RwLock::new(on_optimization_start));
    let on_optimization_end = Arc::new(RwLock::new(on_optimization_end));
    assets_to_transfer
        .par_iter()
        .try_for_each(|(from, to, options)| {
            {
                let mut on_optimization_start = on_optimization_start.write().unwrap();
                on_optimization_start(from, to, options);
            }

            let res = process_file_to(options, from, to);
            if let Err(err) = res.as_ref() {
                tracing::error!("Failed to copy asset {from:?}: {err}");
            }

            {
                let mut on_optimization_end = on_optimization_end.write().unwrap();
                on_optimization_end(from, to, options);
            }

            res.map(|_| ())
        })
}
