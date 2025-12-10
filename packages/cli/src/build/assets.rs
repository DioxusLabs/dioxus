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
use anyhow::{bail, Context};
use const_serialize::{ConstVec, SerializeConst};
use dioxus_cli_opt::AssetManifest;
use manganis::BundledAsset;
use object::{File, Object, ObjectSection, ObjectSymbol, ReadCache, ReadRef, Section, Symbol};
use pdb::FallibleIterator;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};

/// Extract all manganis symbols and their sections from the given object file.
fn manganis_symbols<'a, 'b, R: ReadRef<'a>>(
    file: &'b File<'a, R>,
) -> impl Iterator<Item = (Symbol<'a, 'b, R>, Section<'a, 'b, R>)> + 'b {
    file.symbols()
        .filter(|symbol| {
            if let Ok(name) = symbol.name() {
                looks_like_manganis_symbol(name)
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

fn looks_like_manganis_symbol(name: &str) -> bool {
    name.contains("__MANGANIS__")
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

fn eval_walrus_global_expr(module: &walrus::Module, expr: &walrus::ConstExpr) -> Option<u64> {
    match expr {
        walrus::ConstExpr::Value(walrus::ir::Value::I32(value)) => Some(*value as u64),
        walrus::ConstExpr::Value(walrus::ir::Value::I64(value)) => Some(*value as u64),
        walrus::ConstExpr::Global(id) => {
            let global = module.globals.get(*id);
            if let walrus::GlobalKind::Local(pointer) = &global.kind {
                eval_walrus_global_expr(module, pointer)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Find the offsets of any manganis symbols in the wasm file.
fn find_wasm_symbol_offsets<'a, R: ReadRef<'a>>(
    file_contents: &[u8],
    file: &File<'a, R>,
) -> Result<Vec<u64>> {
    let Some(section) = file
        .sections()
        .find(|section| section.name() == Ok("<data>"))
    else {
        tracing::error!("Failed to find <data> section in WASM file");
        return Ok(Vec::new());
    };
    let Some((_, section_range_end)) = section.file_range() else {
        tracing::error!("Failed to find file range for <data> section in WASM file");
        return Ok(Vec::new());
    };
    let section_size = section.data()?.len() as u64;
    let section_start = section_range_end - section_size;
    
    tracing::warn!("WASM <data> section: start={:#x}, end={:#x}, size={}", section_start, section_range_end, section_size);

    // Parse the wasm file to find the globals
    let module = walrus::Module::from_buffer(file_contents)
        .context("Failed to parse WASM module from file contents")?;
    
    // Check memory configuration
    for mem in module.memories.iter() {
        tracing::warn!("Memory: initial_pages={}, max_pages={:?}, shared={}", 
            mem.initial, mem.maximum, mem.shared);
    }

    // Collect file offsets for all data segments using wasmparser
    let reader = wasmparser::DataSectionReader::new(wasmparser::BinaryReader::new(
        &file_contents[section_start as usize..section_range_end as usize],
        0,
    ))
    .context("Failed to create WASM data section reader")?;

    let mut data_segment_file_offsets = Vec::new();
    let mut segment_count = 0;
    for data in reader.into_iter() {
        let data = data.context("Failed to read data segment from WASM data section")?;
        let data_start_offset = (data.data.as_ptr() as u64)
            .checked_sub(file_contents.as_ptr() as u64)
            .expect("Data section start offset should be within the file contents");
        data_segment_file_offsets.push(data_start_offset);
        
        let kind_str = match &data.kind {
            wasmparser::DataKind::Active { memory_index, offset_expr } => {
                format!("Active(mem={}, offset_expr_len={})", memory_index, offset_expr.get_binary_reader().bytes_remaining())
            },
            wasmparser::DataKind::Passive => "Passive".to_string(),
        };
        
        tracing::warn!(
            "Wasmparser segment {}: kind={}, data_len={}, file_offset={:#x}",
            segment_count,
            kind_str,
            data.data.len(),
            data_start_offset
        );
        segment_count += 1;
    }
    
    tracing::warn!("Total segments found via wasmparser: {}", data_segment_file_offsets.len());

    // Build a map of segment memory ranges. For Active segments, use their offset.
    // For Passive segments, we need to determine where they will be placed at runtime.
    // With atomics/bulk-memory, passive segments are typically loaded at __data_end.
    // We'll look for __data_end or similar globals to find the base address.
    let mut passive_base: Option<u64> = None;
    for export in module.exports.iter() {
        if export.name == "__data_end" || export.name == "__heap_base" {
            if let walrus::ExportItem::Global(global) = export.item {
                if let walrus::GlobalKind::Local(expr) = &module.globals.get(global).kind {
                    passive_base = eval_walrus_global_expr(&module, expr);
                    tracing::debug!("Found {} = {:?}", export.name, passive_base);
                    break;
                }
            }
        }
    }

    // Build segment memory ranges
    struct SegmentInfo {
        memory_start: u64,
        memory_end: u64,
        file_offset: u64,
        is_active: bool,
    }
    
    let mut segments: Vec<SegmentInfo> = Vec::new();
    let mut cumulative_passive_offset = passive_base.unwrap_or(0);
    
    let mut walrus_segment_count = 0;
    for (i, data) in module.data.iter().enumerate() {
        walrus_segment_count += 1;
        let file_offset = data_segment_file_offsets.get(i).copied().unwrap_or(0);
        let data_len = data.value.len() as u64;
        
        match &data.kind {
            walrus::DataKind::Active { offset, .. } => {
                let memory_start = eval_walrus_global_expr(&module, offset).unwrap_or(0);
                segments.push(SegmentInfo {
                    memory_start,
                    memory_end: memory_start + data_len,
                    file_offset,
                    is_active: true,
                });
                tracing::debug!(
                    "Active segment {}: memory [{:#x} - {:#x}), file_offset={:#x}, len={}",
                    i, memory_start, memory_start + data_len, file_offset, data_len
                );
            }
            walrus::DataKind::Passive => {
                // For passive segments, assume they are placed sequentially starting at passive_base
                // This is an approximation - the actual placement depends on memory.init calls
                segments.push(SegmentInfo {
                    memory_start: cumulative_passive_offset,
                    memory_end: cumulative_passive_offset + data_len,
                    file_offset,
                    is_active: false,
                });
                tracing::debug!(
                    "Passive segment {}: memory [{:#x} - {:#x}), file_offset={:#x}, len={}",
                    i, cumulative_passive_offset, cumulative_passive_offset + data_len, file_offset, data_len
                );
                cumulative_passive_offset += data_len;
            }
        }
    }
    
    tracing::warn!("Walrus found {} data segments total", walrus_segment_count);
    tracing::warn!("Segment mismatch check: wasmparser={}, walrus={}", data_segment_file_offsets.len(), walrus_segment_count);

    // Check if we have only passive segments (atomics enabled)
    let all_passive = segments.iter().all(|s| !s.is_active);
    
    if all_passive && !segments.is_empty() {
        // With atomics, we can't use symbol addresses to locate assets reliably
        // Count the expected number of assets first
        let symbol_count = module.exports.iter()
            .filter(|e| looks_like_manganis_symbol(&e.name))
            .count();
        
        tracing::warn!("Detected atomics-enabled WASM (all passive data segments)");
        tracing::warn!("Using direct segment data offsets for {} potential assets", symbol_count);
        
        let mut offsets = Vec::new();
        let asset_size = BundledAsset::MEMORY_LAYOUT.size();
        
        // Try to find assets in the first large segment (usually Segment 1)
        for seg in &segments {
            if seg.memory_end - seg.memory_start >= asset_size as u64 {
                // This segment is large enough to potentially contain assets
                // Try to deserialize from the beginning of this segment
                let seg_file_start = seg.file_offset as usize;
                let seg_file_end = seg_file_start + (seg.memory_end - seg.memory_start) as usize;
                
                if seg_file_start < file_contents.len() && seg_file_end <= file_contents.len() {
                    let seg_data = &file_contents[seg_file_start..seg_file_end];
                    
                    // Search for patterns that look like BundledAsset starts
                    // BundledAsset starts with const-serialize metadata
                    for offset in (0..seg_data.len().saturating_sub(asset_size)).step_by(256) {
                        if offset + asset_size > seg_data.len() {
                            break;
                        }
                        
                        let file_offset = seg_file_start + offset;
                        offsets.push(file_offset as u64);
                        tracing::debug!("Guessing asset at offset {:#x} in passive segment", file_offset);
                        
                        if offsets.len() >= symbol_count {
                            break;
                        }
                    }
                    
                    if offsets.len() >= symbol_count {
                        break;
                    }
                }
            }
        }
        
        if !offsets.is_empty() {
            tracing::warn!("Returning {} potential asset offsets", offsets.len());
            return Ok(offsets);
        }
    }

    let mut offsets = Vec::new();

    for export in module.exports.iter() {
        if !looks_like_manganis_symbol(&export.name) {
            continue;
        }

        let walrus::ExportItem::Global(global) = export.item else {
            tracing::debug!("Symbol {:?} is not a global, skipping", export.name);
            continue;
        };

        let walrus::GlobalKind::Local(pointer) = module.globals.get(global).kind else {
            tracing::debug!("Symbol {:?} is not a local global, skipping", export.name);
            continue;
        };

        let Some(virtual_address) = eval_walrus_global_expr(&module, &pointer) else {
            tracing::error!(
                "Found __MANGANIS__ symbol {:?} in WASM file, but the global expression could not be evaluated",
                export.name
            );
            continue;
        };
        
        tracing::debug!(
            "Found MANGANIS symbol {:?} with virtual address {:#x}",
            export.name,
            virtual_address
        );

        // Search all segments for this virtual address
        let mut found = false;
        for seg in &segments {
            if virtual_address >= seg.memory_start && virtual_address < seg.memory_end {
                let relative_offset = virtual_address - seg.memory_start;
                let file_offset = seg.file_offset + relative_offset;
                offsets.push(file_offset);
                found = true;
                tracing::debug!(
                    "Found __MANGANIS__ symbol {:?} at file offset {:#x} (segment type: {})",
                    export.name,
                    file_offset,
                    if seg.is_active { "active" } else { "passive" }
                );
                break;
            }
        }

        if !found {
            // Log detailed debug info but don't panic - the asset might still work
            tracing::warn!(
                "Found __MANGANIS__ symbol {:?} (addr: {:#x}) in WASM file, but no segment range matched",
                export.name,
                virtual_address
            );
            tracing::warn!("Available segments:");
            for (i, seg) in segments.iter().enumerate() {
                tracing::warn!(
                    "  Segment {}: {} [{:#x} - {:#x}) file_offset={:#x}",
                    i,
                    if seg.is_active { "Active" } else { "Passive" },
                    seg.memory_start,
                    seg.memory_end,
                    seg.file_offset
                );
            }
        }
    }

    Ok(offsets)
}

/// Find all assets in the given file, hash them, and write them back to the file.
/// Then return an `AssetManifest` containing all the assets found in the file.
pub(crate) async fn extract_assets_from_file(path: impl AsRef<Path>) -> Result<AssetManifest> {
    let path = path.as_ref();
    let mut file = open_file_for_writing_with_timeout(
        path,
        std::fs::OpenOptions::new().write(true).read(true),
    )
    .await?;

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
            tracing::debug!(
                "Found asset at offset {offset}: {:?}",
                bundled_asset.absolute_source_path()
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
        tracing::debug!("Writing asset to offset {offset}: {:?}", asset);
        let new_data = ConstVec::new();
        let new_data = const_serialize::serialize_const(asset, new_data);

        file.seek(std::io::SeekFrom::Start(offset))?;
        // Write the modified binary data back to the file
        file.write_all(new_data.as_ref())?;
    }

    // Ensure the file is flushed to disk
    file.sync_all()
        .context("Failed to sync file after writing assets")?;

    // If the file is a macos binary, we need to re-sign the modified binary
    if object_file.format() == object::BinaryFormat::MachO && !assets.is_empty() {
        // Spawn the codesign command to re-sign the binary
        let output = std::process::Command::new("codesign")
            .arg("--force")
            .arg("--sign")
            .arg("-") // Sign with an empty identity
            .arg(path)
            .output()
            .context("Failed to run codesign - is `codesign` in your path?")?;

        if !output.status.success() {
            bail!(
                "Failed to re-sign the binary with codesign after finalizing the assets: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    // Finally, create the asset manifest
    let mut manifest = AssetManifest::default();
    for asset in assets {
        manifest.insert_asset(asset);
    }

    Ok(manifest)
}

/// Try to open a file for writing, retrying if the file is already open by another process.
///
/// This is useful on windows where antivirus software might grab the executable before we have a chance to read it.
async fn open_file_for_writing_with_timeout(
    file: &Path,
    options: &mut std::fs::OpenOptions,
) -> Result<std::fs::File> {
    let start_time = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(5);
    loop {
        match options.open(file) {
            Ok(file) => return Ok(file),
            Err(e) => {
                if cfg!(windows) && e.raw_os_error() == Some(32) && start_time.elapsed() < timeout {
                    // File is already open, wait and retry
                    tracing::trace!(
                        "Failed to open file because another process is using it. Retrying..."
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                } else {
                    return Err(e.into());
                }
            }
        }
    }
}
