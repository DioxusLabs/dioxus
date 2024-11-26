use anyhow::Context;
use manganis_core::linker::LinkSection;
use manganis_core::BundledAsset;
use object::{read::archive::ArchiveFile, File as ObjectFile, Object, ObjectSection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{collections::HashMap, path::PathBuf};

mod css;
mod file;
mod folder;
mod image;
mod js;
mod json;

pub(crate) use file::process_file_to;

/// A manifest of all assets collected from dependencies
///
/// This will be filled in primarily by incremental compilation artifacts.
#[derive(Debug, PartialEq, Default, Clone, Serialize, Deserialize)]
pub(crate) struct AssetManifest {
    /// Map of bundled asset name to the asset itself
    pub(crate) assets: HashMap<PathBuf, BundledAsset>,
}

impl AssetManifest {
    #[allow(dead_code)]
    pub(crate) fn load_from_file(path: &Path) -> anyhow::Result<Self> {
        let src = std::fs::read_to_string(path)?;

        serde_json::from_str(&src)
            .with_context(|| format!("Failed to parse asset manifest from {path:?}\n{src}"))
    }

    /// Fill this manifest with a file object/rlib files, typically extracted from the linker intercepted
    pub(crate) fn add_from_object_path(&mut self, path: &Path) -> anyhow::Result<()> {
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
                    .insert(asset.absolute_source_path().into(), asset);
                buffer = remaining_buffer;
            }
        }

        Ok(())
    }
}
