use anyhow::Context;
use manganis::AssetOptions;
use manganis_core::linker::LinkSection;
use manganis_core::BundledAsset;
use object::{read::archive::ArchiveFile, File as ObjectFile, Object, ObjectSection};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

mod css;
mod file;
mod folder;
mod image;
mod js;
mod json;

pub use file::process_file_to;

/// A manifest of all assets collected from dependencies
///
/// This will be filled in primarily by incremental compilation artifacts.
#[derive(Debug, PartialEq, Default, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    /// Map of bundled asset name to the asset itself
    assets: HashMap<PathBuf, HashSet<BundledAsset>>,
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

        let bundled_asset = manganis::macro_helpers::create_bundled_asset(output_path_str, options);

        self.assets
            .entry(asset_path.to_path_buf())
            .or_default()
            .insert(bundled_asset);

        Ok(bundled_asset)
    }

    /// Get any assets that are tied to a specific source file
    pub fn get_assets_for_source(&self, path: &Path) -> Option<&HashSet<BundledAsset>> {
        self.assets.get(path)
    }

    /// Iterate over all the assets in the manifest
    pub fn assets(&self) -> impl Iterator<Item = &BundledAsset> {
        self.assets.values().flat_map(|assets| assets.iter())
    }

    #[allow(dead_code)]
    pub fn load_from_file(path: &Path) -> anyhow::Result<Self> {
        let src = std::fs::read_to_string(path)?;

        serde_json::from_str(&src)
            .with_context(|| format!("Failed to parse asset manifest from {path:?}\n{src}"))
    }

    /// Fill this manifest with a file object/rlib files, typically extracted from the linker intercepted
    pub fn add_from_object_path(&mut self, path: &Path) -> anyhow::Result<()> {
        let data = std::fs::read(path)?;

        match path.extension().and_then(|ext| ext.to_str()) {
            // Parse an rlib as a collection of objects
            Some("rlib") => {
                if let Ok(archive) = object::read::archive::ArchiveFile::parse(&*data) {
                    self.add_from_archive_file(&archive, &data)?;
                }
            }
            _ => {
                if let Ok(object) = object::File::parse(&*data) {
                    self.add_from_object_file(&object)?;
                }
            }
        }

        Ok(())
    }

    /// Fill this manifest from an rlib / ar file that contains many object files and their entries
    fn add_from_archive_file(&mut self, archive: &ArchiveFile, data: &[u8]) -> object::Result<()> {
        // Look through each archive member for object files.
        // Read the archive member's binary data (we know it's an object file)
        // And parse it with the normal `object::File::parse` to find the manganis string.
        for member in archive.members() {
            let member = member?;
            let name = String::from_utf8_lossy(member.name()).to_string();

            // Check if the archive member is an object file and parse it.
            if name.ends_with(".o") {
                let data = member.data(data)?;
                let object = object::File::parse(data)?;
                _ = self.add_from_object_file(&object);
            }
        }

        Ok(())
    }

    /// Fill this manifest with whatever tables might come from the object file
    fn add_from_object_file(&mut self, obj: &ObjectFile) -> anyhow::Result<()> {
        for section in obj.sections() {
            let Ok(section_name) = section.name() else {
                continue;
            };

            // Check if the link section matches the asset section for one of the platforms we support. This may not be the current platform if the user is cross compiling
            let matches = LinkSection::ALL
                .iter()
                .any(|x| x.link_section == section_name);

            if !matches {
                continue;
            }

            let bytes = section
                .uncompressed_data()
                .context("Could not read uncompressed data from object file")?;

            let mut buffer = const_serialize::ConstReadBuffer::new(&bytes);
            while let Some((remaining_buffer, asset)) =
                const_serialize::deserialize_const!(BundledAsset, buffer)
            {
                self.assets
                    .entry(PathBuf::from(asset.absolute_source_path()))
                    .or_default()
                    .insert(asset);
                buffer = remaining_buffer;
            }
        }

        Ok(())
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
