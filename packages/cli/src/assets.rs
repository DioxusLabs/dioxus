use anyhow::Context;
use manganis_core::{LinkSection, ResourceAsset};
use object::{read::archive::ArchiveFile, File as ObjectFile, Object, ObjectSection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{collections::HashMap, path::PathBuf};

/// A manifest of all assets collected from dependencies
///
/// This will be filled in primarily by incremental compilation artifacts.
#[derive(Debug, PartialEq, Default, Clone, Serialize, Deserialize)]
pub(crate) struct AssetManifest {
    /// Map of bundled asset name to the asset itself
    pub(crate) assets: HashMap<PathBuf, ResourceAsset>,
}

impl AssetManifest {
    pub(crate) fn load_from_file(path: &Path) -> anyhow::Result<Self> {
        let src = std::fs::read_to_string(path)
            .context("Failed to read asset manifest from filesystem")?;
        serde_json::from_str(&src)
            .with_context(|| format!("Failed to parse asset manifest from {path:?}\n{src}"))
    }

    /// Fill this manifest with a file object/rlib files, typically extracted from the linker intercepted
    pub(crate) fn add_from_object_path(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let Some(ext) = path.extension() else {
            return Ok(());
        };

        let Some(ext) = ext.to_str() else {
            return Ok(());
        };

        let data = std::fs::read(path.clone())?;

        match ext {
            // Parse an unarchived object file
            "o" => {
                if let Ok(object) = object::File::parse(&*data) {
                    self.add_from_object_file(&object)?;
                }
            }

            // Parse an rlib as a collection of objects
            "rlib" => {
                if let Ok(archive) = object::read::archive::ArchiveFile::parse(&*data) {
                    self.add_from_archive_file(&archive, &data)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Fill this manifest from an rlib / ar file that contains many object files and their entryies
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

            let as_str = std::str::from_utf8(&bytes)
                .context("object file contained non utf8 encoding")?
                .chars()
                .filter(|c| !c.is_control())
                .collect::<String>();

            let assets = serde_json::Deserializer::from_str(&as_str).into_iter::<ResourceAsset>();
            for as_resource in assets.flatten() {
                // Some platforms (e.g. macOS) start the manganis section with a null byte, we need to filter that out before we deserialize the JSON
                self.assets
                    .insert(as_resource.absolute.clone(), as_resource);
            }
        }

        Ok(())
    }
}
