//! The dioxus asset system.
//!
//! This module provides functionality for extracting assets from a binary file and then writing back
//! their asset hashes directly into the binary file. Previously, we performed asset hashing in the
//! `asset!()` macro. The new system, implemented here, instead performs the hashing at build time,
//! which provides more flexibility in the asset processing pipeline.
//!
//! We chose to implement this approach since assets might reference each other which means we minimally
//! need to parse the asset to create a unique hash for each asset before they are used in the application.
//! The hashes are used both for cache busting the asset in the browser and to cache the asset optimization
//! process in the build system.
//!
//! We use the same lessons learned from the hot-patching engine which parses the binary file and its
//! symbol table to find symbols that match the `__MANGANIS__` prefix. These symbols are ideally data
//! symbols and contain the BundledAsset data type which implements ConstSerialize and ConstDeserialize.
//!
//! When the binary is built, the `dioxus asset!()` macro will emit its metadata into the __MANGANIS__
//! symbols, which we process here. After reading the metadata directly from the executable, we then
//! hash it and write the hash directly into the binary file.
//!
//! During development, we can skip this step for most platforms since local paths are sufficient
//! for asset loading. However, for WASM and for production builds, we need to ensure that assets
//! can be found relative to the current exe. Unfortunately, on android, the `current_exe` path is wrong,
//! so the assets are resolved against the "asset root" - which is covered by the asset loader crate.
//!
//! Finding the __MANGANIS__ symbols is not quite straightforward when hotpatching, especially on WASM
//! since we build and link the module as relocatable, which is not a stable WASM proposal. In this
//! implementation, we handle both the non-PIE *and* PIC cases which are rather bespoke to our whole
//! build system.

use std::{
    io::{Cursor, Read, Seek, Write},
    path::{Path, PathBuf},
};

use crate::Result;
use anyhow::Context;
use const_serialize::{ConstVec, SerializeConst};
use dioxus_cli_opt::AssetManifest;
use manganis::BundledAsset;
use object::{File, Object, ObjectSection, ObjectSymbol, ReadCache, ReadRef, Section, Symbol};
use pdb::FallibleIterator;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use wasmparser::BinaryReader;

/// Extract all manganis symbols and their sections from the given object file.
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

/// Find the offsets of any manganis symbols in the given file.
fn find_symbol_offsets<'a, R: ReadRef<'a>>(
    path: &Path,
    file_contents: &[u8],
    file: &File<'a, R>,
) -> Result<Vec<u64>> {
    let pdb_file = find_pdb_file(path);

    match file.format() {
        // We need to handle dynamic offsets in wasm files differently
        object::BinaryFormat::Wasm => find_wasm_symbol_offsets(file_contents, file),
        // Windows puts the symbol information in a PDB file alongside the executable.
        // If this is a windows PE file and we found a PDB file, we will use that to find the symbol offsets.
        object::BinaryFormat::Pe if pdb_file.is_some() => {
            find_pdb_symbol_offsets(&pdb_file.unwrap())
        }
        // Otherwise, look for manganis symbols in the object file.
        _ => find_native_symbol_offsets(file),
    }
}

/// Find the pdb file matching the executable file.
fn find_pdb_file(path: &Path) -> Option<PathBuf> {
    let mut pdb_file = path.with_extension("pdb");
    // Also try to find it in the same directory as the executable with _'s instead of -'s
    if let Some(file_name) = pdb_file.file_name() {
        let new_file_name = file_name.to_string_lossy().replace('-', "_");
        let altrnate_pdb_file = pdb_file.with_file_name(new_file_name);
        // Keep the most recent pdb file
        match (pdb_file.metadata(), altrnate_pdb_file.metadata()) {
            (Ok(pdb_metadata), Ok(alternate_metadata)) => {
                if let (Ok(pdb_modified), Ok(alternate_modified)) =
                    (pdb_metadata.modified(), alternate_metadata.modified())
                {
                    if pdb_modified < alternate_modified {
                        pdb_file = altrnate_pdb_file;
                    }
                }
            }
            (Err(_), Ok(_)) => {
                pdb_file = altrnate_pdb_file;
            }
            _ => {}
        }
    }
    if pdb_file.exists() {
        Some(pdb_file)
    } else {
        None
    }
}

/// Find the offsets of any manganis symbols in a pdb file.
fn find_pdb_symbol_offsets(pdb_file: &Path) -> Result<Vec<u64>> {
    let pdb_file_handle = std::fs::File::open(pdb_file)?;
    let mut pdb_file = pdb::PDB::open(pdb_file_handle).context("Failed to open PDB file")?;
    let Ok(Some(sections)) = pdb_file.sections() else {
        tracing::error!("Failed to read sections from PDB file");
        return Ok(Vec::new());
    };
    let global_symbols = pdb_file
        .global_symbols()
        .context("Failed to read global symbols from PDB file")?;
    let address_map = pdb_file
        .address_map()
        .context("Failed to read address map from PDB file")?;
    let mut symbols = global_symbols.iter();
    let mut addresses = Vec::new();
    while let Ok(Some(symbol)) = symbols.next() {
        let Ok(pdb::SymbolData::Public(data)) = symbol.parse() else {
            continue;
        };
        let Some(rva) = data.offset.to_section_offset(&address_map) else {
            continue;
        };

        let name = data.name.to_string();
        if name.contains("__MANGANIS__") {
            let section = sections
                .get(rva.section as usize - 1)
                .expect("Section index out of bounds");

            addresses.push((section.pointer_to_raw_data + rva.offset) as u64);
        }
    }
    Ok(addresses)
}

/// Find the offsets of any manganis symbols in a native object file.
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

/// Find the offsets of any manganis symbols in the wasm file.
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
        let section_size = section.data()?.len() as u64;
        let section_start = section_range_end - section_size;
        // Translate the section_relative_address to the file offset
        // WASM files have a section address of 0 in object, reparse the data section with wasmparser
        // to get the correct address and section start
        let reader = wasmparser::DataSectionReader::new(BinaryReader::new(
            &file_contents[section_start as usize..section_range_end as usize],
            0,
        ))
        .context("Failed to create WASM data section reader")?;
        let main_memory = reader
            .into_iter()
            .next()
            .context("Failed find main memory from WASM data section")?
            .context("Failed to read main memory from WASM data section")?;
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

/// Find all assets in the given file, hash them, and write them back to the file.
/// Then return an `AssetManifest` containing all the assets found in the file.
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
