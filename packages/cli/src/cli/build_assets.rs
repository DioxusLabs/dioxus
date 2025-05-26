use std::{
    fs::create_dir_all,
    io::{Cursor, Read, Seek, Write},
    path::{Path, PathBuf},
};

use crate::{Result, StructuredOutput};
use clap::Parser;
use const_serialize::{ConstVec, SerializeConst};
use dioxus_cli_opt::{process_file_to, AssetManifest};
use manganis::BundledAsset;
use object::{File, Object, ObjectSection, ObjectSymbol, ReadCache, ReadRef};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use tracing::debug;
use wasmparser::BinaryReader;

#[derive(Clone, Debug, Parser)]
pub struct BuildAssets {
    /// The source executable to build assets for.
    pub(crate) executable: PathBuf,

    /// The destination directory for the assets.
    pub(crate) destination: PathBuf,
}

impl BuildAssets {
    pub async fn run(self) -> Result<StructuredOutput> {
        let manifest = extract_assets_from_file(&self.executable)?;

        create_dir_all(&self.destination)?;
        for asset in manifest.assets() {
            let source_path = PathBuf::from(asset.absolute_source_path());
            let destination_path = self.destination.join(asset.bundled_path());
            debug!(
                "Processing asset {} --> {} {:#?}",
                source_path.display(),
                destination_path.display(),
                asset
            );
            process_file_to(asset.options(), &source_path, &destination_path)?;
        }

        Ok(StructuredOutput::Success)
    }
}

fn find_symbol_offsets<'a, R: ReadRef<'a>>(
    file_contents: &[u8],
    file: &File<'a, R>,
) -> Result<Vec<u64>> {
    let mut offsets = Vec::new();
    {
        for symbol in file.symbols() {
            let Ok(name) = symbol.name() else {
                continue;
            };
            if name.contains("__MANGANIS__") {
                let virtual_address = symbol.address();

                let section = symbol.section_index();
                if let Some(section) = section {
                    let Ok(section) = file.section_by_index(section) else {
                        continue;
                    };
                    let Some((section_range_start, section_range_end)) = section.file_range()
                    else {
                        continue;
                    };
                    let section_size = section.data().unwrap().len() as u64;
                    let section_start = section_range_end - section_size;
                    // Translate the section_relative_address to the file offset
                    let (section_address, section_offset) =
                        if file.format() == object::BinaryFormat::Wasm {
                            // WASM files have a section address of 0 in object, reparse the data section with wasmparser
                            // to get the correct address and section start
                            let reader = wasmparser::DataSectionReader::new(BinaryReader::new(
                                &file_contents[section_start as usize..section_range_end as usize],
                                0,
                            ))
                            .unwrap();
                            let main_memory = reader.into_iter().next().unwrap().unwrap();
                            let main_memory_offset = match main_memory.kind {
                                wasmparser::DataKind::Active { offset_expr, .. } => {
                                    match offset_expr.get_operators_reader().into_iter().next() {
                                        Some(Ok(wasmparser::Operator::I32Const { value })) => {
                                            value as u64
                                        }
                                        Some(Ok(wasmparser::Operator::I64Const { value })) => {
                                            value as u64
                                        }
                                        _ => continue,
                                    }
                                }
                                _ => continue,
                            };
                            // main_memory.data is a slice somewhere in file_contents. Find out the offset in the file
                            let data_start_offset =
                                main_memory.data.as_ptr() as u64 - file_contents.as_ptr() as u64;
                            (main_memory_offset, data_start_offset)
                        } else {
                            (section.address(), section_range_start)
                        };
                    let section_relative_address = virtual_address - section_address;
                    let file_offset = section_offset + section_relative_address;
                    offsets.push(file_offset);
                }
            }
        }
    }

    Ok(offsets)
}

pub(crate) fn extract_assets_from_file(path: impl AsRef<Path>) -> Result<AssetManifest> {
    let path = path.as_ref();
    let mut file = std::fs::File::options().write(true).read(true).open(path)?;
    let mut file_contents = Vec::new();
    file.read_to_end(&mut file_contents)?;
    let mut reader = Cursor::new(&file_contents);
    let read_cache = ReadCache::new(&mut reader);
    let object_file = object::File::parse(&read_cache)?;
    let offsets = find_symbol_offsets(&file_contents, &object_file)?;

    let mut assets = Vec::new();

    // Read each asset from the data section using the offsets
    for offset in offsets.iter().copied() {
        file.seek(std::io::SeekFrom::Start(offset))?;
        let mut data_in_range = vec![0; BundledAsset::MEMORY_LAYOUT.size()];
        file.read_exact(&mut data_in_range)?;

        let buffer = const_serialize::ConstReadBuffer::new(&data_in_range);

        if let Some((_, bundled_asset)) = const_serialize::deserialize_const!(BundledAsset, buffer)
        {
            assets.push(bundled_asset);
        } else {
            tracing::warn!("Found an asset at offset {offset} that could not be deserialized. This may be caused by a mismatch between your dioxus and dioxus-cli versions.");
        }
    }

    // Add the hash to each asset in parallel
    assets
        .par_iter_mut()
        .for_each(dioxus_cli_opt::add_hash_to_asset);

    // Write back the assets to the binary file
    for (offset, asset) in offsets.into_iter().zip(&assets) {
        let new_data = ConstVec::new();
        let new_data = const_serialize::serialize_const(asset, new_data);

        file.seek(std::io::SeekFrom::Start(offset))?;
        // Write the modified binary data back to the file
        file.write_all(new_data.as_ref())?;
    }

    // If the file is a macos binary, we need to re-sign the modified binary
    if object_file.format() == object::BinaryFormat::MachO {
        // Spawn the codesign command to re-sign the binary
        let output = std::process::Command::new("codesign")
            .arg("--force")
            .arg("--sign")
            .arg("-") // Sign with an empty identity
            .arg(path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to re-sign the binary with codesign after finalizing the assets: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
    }

    // Finally, create the asset manifest
    let mut manifest = AssetManifest::default();
    for asset in assets {
        manifest.insert_asset(asset);
    }

    Ok(manifest)
}
