use manganis_core::linker::LinkSection;
use manganis_core::BundledMetadata;
use object::{read::archive::ArchiveFile, File as ObjectFile, Object, ObjectSection};
use serde::{Deserialize, Serialize};

/// Fill this manifest with whatever tables might come from the object file
fn collect_elements_metadata(&mut self, obj: &ObjectFile) -> anyhow::Result<HasMap<ConstStr, ConstStr>> {
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
            const_serialize::deserialize_const!(BundledMetadata, buffer)
        {
            self.assets
                .insert(asset.absolute_source_path().into(), asset);
            buffer = remaining_buffer;
        }
    }

    Ok(())
}
