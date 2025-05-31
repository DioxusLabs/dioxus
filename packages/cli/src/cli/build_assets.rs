use std::{
    collections::HashMap,
    fs::create_dir_all,
    io::{Cursor, Read, Seek, Write},
    path::{Path, PathBuf},
};

use crate::{Result, StructuredOutput};
use clap::Parser;
use const_serialize::{ConstVec, SerializeConst};
use dioxus_cli_opt::{process_file_to, AssetManifest};
use manganis::BundledAsset;
use object::{File, Object, ObjectSection, ObjectSymbol, ReadCache, ReadRef, Section, Symbol};
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

fn manganis_symbols<'a, 'b, R: ReadRef<'a>>(
    file: &'b File<'a, R>,
) -> impl Iterator<Item = (Symbol<'a, 'b, R>, Section<'a, 'b, R>)> + 'b {
    file.symbols()
        .filter(|symbol| {
            if let Ok(name) = symbol.name() {
                name.contains("__MANGANIS__")
            } else {
                false
            }
        })
        .filter_map(move |symbol| {
            let section_index = symbol.section_index()?;
            let section = file.section_by_index(section_index).ok()?;
            Some((symbol, section))
        })
}

fn find_symbol_offsets<'a, R: ReadRef<'a>>(
    path: &Path,
    file_contents: &[u8],
    file: &File<'a, R>,
) -> Result<Vec<u64>> {
    use pdb::FallibleIterator;

    // If there is a pdb file in the same directory as the executable, use it to find the symbols
    let pdb_file = path.with_extension("pdb");
    // replace any -'s in the filename with _'s
    let pdb_file = pdb_file.with_file_name(pdb_file.file_name().unwrap().to_str().unwrap().replace("-", "_"));
    tracing::info!("Looking for PDB file at {}", pdb_file.display());

    if file.format() == object::BinaryFormat::Wasm {
        find_wasm_symbol_offsets(file_contents, file)
    } else if pdb_file.exists() {
        tracing::info!("Found PDB file at {}", pdb_file.display());
        let pdb_file_handle = std::fs::File::open(pdb_file).unwrap();
        let mut pdb_file = pdb::PDB::open(pdb_file_handle).unwrap();
        let Ok(Some(sections)) = pdb_file.sections() else { 
            tracing::error!("Failed to read sections from PDB file");
            return Ok(Vec::new());
        };
        let global_symbols = pdb_file.global_symbols().unwrap();
        let address_map = pdb_file.address_map().unwrap();
        let mut symbols = global_symbols.iter();
        let mut addressses = Vec::new();
        while let Ok(Some(symbol)) = symbols.next() {
            match symbol.parse() {
                Ok(pdb::SymbolData::Public(data)) => {
                    let Some(rva) = data.offset.to_section_offset(&address_map) else {
                        continue;
                    };
                    
                    let name = data.name.to_string();
                    if name.contains("__MANGANIS__") {
                        let section = sections
                            .get(rva.section as usize)
                            .expect("Section index out of bounds");
    
                        tracing::info!("Found public symbol {} at address {:?}", data.name, rva);
                        addressses.push((section.pointer_to_raw_data + rva.offset) as u64); 
                    }
                }

            
                _ => {}
            }
        }
        Ok(addressses)
    } else {
        find_native_symbol_offsets(file)
    }
}

fn find_native_symbol_offsets<'a, R: ReadRef<'a>>(file: &File<'a, R>) -> Result<Vec<u64>> {
    let mut offsets = Vec::new();
    for (symbol, section) in manganis_symbols(file) {
        let virtual_address = symbol.address();

        let Some((section_range_start, _)) = section.file_range() else {
            tracing::error!(
                "Found __MANGANIS__ symbol {:?} in section {}, but the section has no file range",
                symbol.name(),
                section.index()
            );
            continue;
        };
        // Translate the section_relative_address to the file offset
        let section_relative_address: u64 = (virtual_address as i128 - section.address() as i128)
            .try_into()
            .expect("Virtual address should be greater than or equal to section address");
        let file_offset = section_range_start + section_relative_address;
        offsets.push(file_offset);
    }

    Ok(offsets)
}

fn find_wasm_symbol_offsets<'a, R: ReadRef<'a>>(
    file_contents: &[u8],
    file: &File<'a, R>,
) -> Result<Vec<u64>> {
    // Parse the wasm file to find the globals
    let parser = wasmparser::Parser::new(0);

    // All integer literal global values in the wasm file
    let mut global_values = Vec::new();
    for section in parser.parse_all(file_contents) {
        let Ok(wasmparser::Payload::GlobalSection(global_section)) = section else {
            continue;
        };

        global_values = global_section
            .into_iter()
            .map(|global| {
                let global = global.ok()?;
                match global.init_expr.get_operators_reader().into_iter().next() {
                    Some(Ok(wasmparser::Operator::I32Const { value })) => Some(value as u64),
                    Some(Ok(wasmparser::Operator::I64Const { value })) => Some(value as u64),
                    _ => None,
                }
            })
            .collect::<Vec<_>>();
    }
    let mut offsets = Vec::new();

    for (symbol, section) in manganis_symbols(file) {
        let virtual_address = symbol.address();

        let Some((_, section_range_end)) = section.file_range() else {
            tracing::error!(
                "Found __MANGANIS__ symbol {:?} in section {}, but the section has no file range",
                symbol.name(),
                section.index()
            );
            continue;
        };
        let section_size = section.data().unwrap().len() as u64;
        let section_start = section_range_end - section_size;
        // Translate the section_relative_address to the file offset
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
                    Some(Ok(wasmparser::Operator::I32Const { value })) => -value as i128,
                    Some(Ok(wasmparser::Operator::I64Const { value })) => -value as i128,
                    Some(Ok(wasmparser::Operator::GlobalGet { global_index })) => {
                        let Some(value) =
                            global_values.get(global_index as usize).copied().flatten()
                        else {
                            tracing::error!(
                                "Found __MANGANIS__ symbol {:?} in WASM file, but the global index {} is not found",
                                symbol.name(),
                                global_index
                            );
                            continue;
                        };
                        value as i128
                    }
                    offset_expr => {
                        tracing::error!(
                            "Found __MANGANIS__ symbol {:?} in WASM file, but the offset expression is not a constant is is {:?}",
                            symbol.name(),
                            offset_expr
                        );
                        continue;
                    }
                }
            }
            _ => {
                tracing::error!(
                    "Found __MANGANIS__ symbol {:?} in WASM file, but the data section is not active",
                    symbol.name()
                );
                continue;
            }
        };
        // main_memory.data is a slice somewhere in file_contents. Find out the offset in the file
        let data_start_offset = (main_memory.data.as_ptr() as u64)
            .checked_sub(file_contents.as_ptr() as u64)
            .expect("Data section start offset should be within the file contents");
        let section_relative_address: u64 = ((virtual_address as i128) + main_memory_offset)
            .try_into()
            .expect("Virtual address should be greater than or equal to section address");
        let file_offset = data_start_offset + section_relative_address;
        offsets.push(file_offset);
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
    let offsets = find_symbol_offsets(path, &file_contents, &object_file)?;

    let mut assets = Vec::new();

    // Read each asset from the data section using the offsets
    for offset in offsets.iter().copied() {
        file.seek(std::io::SeekFrom::Start(offset))?;
        let mut data_in_range = vec![0; BundledAsset::MEMORY_LAYOUT.size()];
        file.read_exact(&mut data_in_range)?;
        tracing::info!("As str {}", String::from_utf8_lossy(&data_in_range));

        let buffer = const_serialize::ConstReadBuffer::new(&data_in_range);

        if let Some((_, bundled_asset)) = const_serialize::deserialize_const!(BundledAsset, buffer)
        {
            tracing::info!(
                "Found asset at offset {offset} {:?} ",
                bundled_asset
            );
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
