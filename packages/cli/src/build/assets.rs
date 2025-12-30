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
    // __MANGANIS__ = Legacy format (0.7.0-0.7.1, uses const_serialize_07)
    // __ASSETS__ = New format (0.7.2+, uses const_serialize/CBOR)
    name.contains("__MANGANIS__") || name.contains("__ASSETS__")
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
    tracing::debug!("=== WASM Module Analysis ===");
    tracing::debug!("File size: {} bytes", file_contents.len());

    // Log all sections from the object file
    tracing::debug!("--- Object file sections ---");
    for section in file.sections() {
        let name = section.name().unwrap_or("<unknown>");
        let address = section.address();
        let size = section.size();
        let file_range = section.file_range();
        tracing::debug!(
            "  Section: {:?}, address: 0x{:x}, size: {}, file_range: {:?}",
            name,
            address,
            size,
            file_range
        );
    }

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
    tracing::debug!(
        "Data section: start=0x{:x}, end=0x{:x}, size={}",
        section_start,
        section_range_end,
        section_size
    );

    // Translate the section_relative_address to the file offset
    // WASM files have a section address of 0 in object, reparse the data section with wasmparser
    // to get the correct address and section start.
    // Note: We need to reparse just the data section with wasmparser to get the file offset because
    // walrus does not expose the file offset information.
    //
    // With bulk memory operations, there may be MULTIPLE data segments. We need to collect all
    // of them to properly map virtual addresses to file offsets.
    let reader = wasmparser::DataSectionReader::new(wasmparser::BinaryReader::new(
        &file_contents[section_start as usize..section_range_end as usize],
        0,
    ))
    .context("Failed to create WASM data section reader")?;

    // Collect all data segments with their file offsets and sizes
    // Each entry is (file_offset, size)
    tracing::debug!("--- Data segments (wasmparser) ---");
    let mut segment_file_info: Vec<(u64, u64)> = Vec::new();
    for segment in reader.into_iter() {
        let segment = segment.context("Failed to read data segment")?;
        let file_offset = (segment.data.as_ptr() as u64)
            .checked_sub(file_contents.as_ptr() as u64)
            .expect("Data segment should be within file contents");
        let size = segment.data.len() as u64;
        let kind_str = match segment.kind {
            wasmparser::DataKind::Active { .. } => "Active",
            wasmparser::DataKind::Passive => "Passive",
        };
        tracing::debug!(
            "  Segment {}: kind={}, file_offset=0x{:x}, size={}",
            segment_file_info.len(),
            kind_str,
            file_offset,
            size
        );
        segment_file_info.push((file_offset, size));
    }

    if segment_file_info.is_empty() {
        tracing::error!("No data segments found in WASM file");
        return Ok(Vec::new());
    }
    tracing::debug!(
        "Found {} data segments from wasmparser",
        segment_file_info.len()
    );

    // Parse the wasm file to find the globals
    let module = walrus::Module::from_buffer(file_contents).unwrap();
    let mut offsets = Vec::new();

    // Log walrus module structure
    tracing::debug!("--- Walrus module analysis ---");
    tracing::debug!("Memories: {}", module.memories.iter().count());
    tracing::debug!("Tables: {}", module.tables.iter().count());
    tracing::debug!("Functions: {}", module.funcs.iter().count());
    tracing::debug!("Globals: {}", module.globals.iter().count());
    tracing::debug!("Exports: {}", module.exports.iter().count());
    tracing::debug!("Data segments: {}", module.data.iter().count());

    // Log all data segments from walrus
    tracing::debug!("--- Data segments (walrus) ---");
    for (i, data) in module.data.iter().enumerate() {
        match &data.kind {
            walrus::DataKind::Active { memory, offset } => {
                let offset_val = eval_walrus_global_expr(&module, offset);
                tracing::debug!(
                    "  Data segment {}: Active, memory={:?}, offset_expr={:?}, evaluated_offset={:?}, data_len={}",
                    i,
                    memory,
                    offset,
                    offset_val,
                    data.value.len()
                );
            }
            walrus::DataKind::Passive => {
                tracing::debug!(
                    "  Data segment {}: Passive, data_len={}",
                    i,
                    data.value.len()
                );
            }
        }
    }

    // Log all exports (filtering out wasm-bindgen noise)
    tracing::debug!("--- Exports ---");
    let mut wbindgen_count = 0usize;

    // Track key memory layout exports
    let mut data_end: Option<u64> = None;
    let mut heap_base: Option<u64> = None;
    let mut stack_pointer: Option<u64> = None;
    let mut tls_base: Option<u64> = None;

    for export in module.exports.iter() {
        // Skip wasm-bindgen internals to reduce noise
        if export.name.starts_with("__wbindgen") || export.name.starts_with("__wbg") {
            wbindgen_count += 1;
            continue;
        }
        let item_str = match &export.item {
            walrus::ExportItem::Function(id) => format!("Function({:?})", id),
            walrus::ExportItem::Global(id) => {
                let global = module.globals.get(*id);
                let val = match &global.kind {
                    walrus::GlobalKind::Local(expr) => eval_walrus_global_expr(&module, expr),
                    _ => None,
                };

                // Track key symbols for memory layout analysis
                match export.name.as_str() {
                    "__data_end" => data_end = val,
                    "__heap_base" => heap_base = val,
                    "__stack_pointer" => stack_pointer = val,
                    "__tls_base" => tls_base = val,
                    _ => {}
                }

                format!("Global({:?}) = {:?}", id, val)
            }
            walrus::ExportItem::Memory(id) => format!("Memory({:?})", id),
            walrus::ExportItem::Table(id) => format!("Table({:?})", id),
        };
        tracing::debug!("  Export {:?}: {}", export.name, item_str);
    }
    if wbindgen_count > 0 {
        tracing::debug!("  (skipped {} __wbindgen/__wbg exports)", wbindgen_count);
    }

    // Summary of key memory layout values
    tracing::debug!("--- Memory layout summary ---");
    if let Some(v) = stack_pointer {
        tracing::debug!("  __stack_pointer = 0x{:x} ({})", v, v);
    }
    if let Some(v) = heap_base {
        tracing::debug!("  __heap_base = 0x{:x} ({})", v, v);
    }
    if let Some(v) = data_end {
        tracing::debug!("  __data_end = 0x{:x} ({})", v, v);
    }
    if let Some(v) = tls_base {
        tracing::debug!("  __tls_base = 0x{:x} ({})", v, v);
    }

    // Find the main memory offset
    // With bulk memory operations enabled, data segments may be Passive instead of Active.
    // - Active segments: have an offset expression that determines where data goes in linear memory
    // - Passive segments: initialized at runtime via memory.init, no static offset
    let main_memory_walrus = module
        .data
        .iter()
        .next()
        .context("Failed to find main memory in WASM module")?;

    let main_memory_offset = match &main_memory_walrus.kind {
        walrus::DataKind::Active { offset, .. } => {
            // In the hot patch build, the main memory offset is a global from the main module
            // and each global is its own global. Use an offset of 0 if we can't evaluate.
            let evaluated = eval_walrus_global_expr(&module, offset).unwrap_or_default();
            tracing::debug!(
                "Main data segment is Active with offset expression {:?}, evaluated to 0x{:x}",
                offset,
                evaluated
            );
            evaluated
        }
        walrus::DataKind::Passive => {
            // For passive segments (bulk memory operations), there's no static offset expression.
            // The memory.init instruction determines placement at runtime.
            //
            // For Rust/LLVM compiled WASM with bulk memory, static data segments are typically
            // placed starting at 1MB (0x100000) in linear memory, leaving the first 1MB for stack.
            //
            // IMPORTANT: With TLS (Thread Local Storage) support, the first segment may be TLS
            // template data that is NOT part of the contiguous linear memory layout. TLS data
            // is copied per-thread via __wasm_init_tls and should be excluded from our calculations.

            // Check for TLS by looking for __tls_size export
            let mut tls_size: Option<u64> = None;
            let mut found_data_end: Option<u64> = None;
            let mut found_heap_base: Option<u64> = None;
            for export in module.exports.iter() {
                match export.name.as_str() {
                    "__tls_size" => {
                        if let walrus::ExportItem::Global(g) = export.item {
                            if let walrus::GlobalKind::Local(expr) = &module.globals.get(g).kind {
                                tls_size = eval_walrus_global_expr(&module, expr);
                                if let Some(size) = tls_size {
                                    tracing::debug!("Found __tls_size: {} bytes", size);
                                }
                            }
                        }
                    }
                    "__data_end" => {
                        if let walrus::ExportItem::Global(g) = export.item {
                            if let walrus::GlobalKind::Local(expr) = &module.globals.get(g).kind {
                                found_data_end = eval_walrus_global_expr(&module, expr);
                            }
                        }
                    }
                    "__heap_base" => {
                        if let walrus::ExportItem::Global(g) = export.item {
                            if let walrus::GlobalKind::Local(expr) = &module.globals.get(g).kind {
                                found_heap_base = eval_walrus_global_expr(&module, expr);
                            }
                        }
                    }
                    _ => {}
                }
            }

            // If TLS is present and segment 0 matches TLS size, remove it from segment_file_info
            // TLS data is NOT part of the contiguous linear memory - it's a template copied per-thread
            if let Some(tls) = tls_size {
                if !segment_file_info.is_empty() && segment_file_info[0].1 == tls {
                    tracing::debug!(
                        "Segment 0 matches __tls_size ({}), removing TLS segment from file info",
                        tls
                    );
                    segment_file_info.remove(0);
                    tracing::debug!(
                        "Remaining {} non-TLS segments for asset lookup",
                        segment_file_info.len()
                    );
                }
            }

            // Calculate total non-TLS data size
            let total_non_tls_size: u64 = segment_file_info.iter().map(|(_, sz)| sz).sum();
            tracing::debug!("Total non-TLS data size: {} bytes (0x{:x})", total_non_tls_size, total_non_tls_size);

            // For WASM with bulk memory, the data segments contain ONLY initialized data.
            // BSS (zero-initialized data) is NOT stored in segments - it's implicitly zero.
            //
            // The memory layout is:
            //   [0x100000, 0x100000 + initialized_size) = initialized data from segments
            //   [0x100000 + initialized_size, __data_end) = BSS (zeros, not in file)
            //
            // IMPORTANT: When TLS exists, the linker calculates symbol addresses as if TLS
            // data is at 0x100000 followed by main data. But at runtime, TLS is stored
            // separately per-thread via __wasm_init_tls. So the main data segment actually
            // starts at 0x100000 in the file, but symbol addresses include the TLS offset.
            //
            // We need to add TLS size to the base so that:
            //   symbol_address - (0x100000 + tls_size) = correct offset into main data segment
            let tls_offset = tls_size.unwrap_or(0);
            let base = 0x100000u64 + tls_offset;
            tracing::debug!(
                "Using WASM data base: 0x{:x} (0x100000 + tls_size {}), initialized data ends at 0x{:x}, __data_end at {:?}",
                base,
                tls_offset,
                base + total_non_tls_size,
                found_data_end
            );

            tracing::debug!(
                "Main data segment is Passive (bulk memory), using base offset: 0x{:x}",
                base
            );

            base
        }
    };
    tracing::debug!("Using main memory offset: 0x{:x}", main_memory_offset);

    // Calculate total data segment size for sanity checking
    let total_data_size: u64 = segment_file_info.iter().map(|(_, sz)| sz).sum();
    tracing::debug!(
        "Total data segment size (after TLS removal): {} bytes (0x{:x})",
        total_data_size,
        total_data_size
    );

    tracing::debug!("--- Searching for manganis symbols ---");
    for export in module.exports.iter() {
        if !looks_like_manganis_symbol(&export.name) {
            continue;
        }
        tracing::debug!("Found manganis symbol: {:?}", export.name);

        let walrus::ExportItem::Global(global) = export.item else {
            tracing::debug!("  Skipping: export is not a global");
            continue;
        };

        let global_data = module.globals.get(global);
        tracing::debug!(
            "  Global id={:?}, ty={:?}, mutable={}",
            global,
            global_data.ty,
            global_data.mutable
        );

        let walrus::GlobalKind::Local(pointer) = global_data.kind else {
            tracing::debug!("  Skipping: global is not local (is an import)");
            continue;
        };

        let Some(virtual_address) = eval_walrus_global_expr(&module, &pointer) else {
            tracing::error!(
                "Found __MANGANIS__ symbol {:?} in WASM file, but the global expression could not be evaluated. expr={:?}",
                export.name,
                pointer
            );
            continue;
        };
        tracing::debug!("  Virtual address: 0x{:x}", virtual_address);

        // Calculate offset relative to the data base address
        let data_relative_offset =
            match (virtual_address as i128).checked_sub(main_memory_offset as i128) {
                Some(offset) if offset >= 0 => offset as u64,
                _ => {
                    tracing::error!(
                        "Virtual address 0x{:x} is below main memory offset 0x{:x}",
                        virtual_address,
                        main_memory_offset
                    );
                    continue;
                }
            };

        // Find which segment this offset falls into
        // Segments are laid out contiguously in memory: [0, size0), [size0, size0+size1), etc.
        let mut cumulative_offset = 0u64;
        let mut file_offset = None;
        tracing::debug!(
            "  Looking for data_relative_offset 0x{:x} in {} segments",
            data_relative_offset,
            segment_file_info.len()
        );
        for (i, (seg_file_offset, seg_size)) in segment_file_info.iter().enumerate() {
            tracing::debug!(
                "    Segment {}: file_offset=0x{:x}, size={}, memory_range=[0x{:x}, 0x{:x})",
                i,
                seg_file_offset,
                seg_size,
                cumulative_offset,
                cumulative_offset + seg_size
            );
            if data_relative_offset < cumulative_offset + seg_size {
                // Found the segment - calculate offset within it
                let offset_in_segment = data_relative_offset - cumulative_offset;
                file_offset = Some(seg_file_offset + offset_in_segment);
                tracing::debug!(
                    "  MATCH: Data relative offset: 0x{:x}, in segment {}, offset_in_segment: 0x{:x}, file_offset: 0x{:x}",
                    data_relative_offset,
                    i,
                    offset_in_segment,
                    seg_file_offset + offset_in_segment
                );

                // Dump the raw bytes at this file offset to verify
                let fo = (seg_file_offset + offset_in_segment) as usize;
                if fo + 64 <= file_contents.len() {
                    let preview: Vec<String> = file_contents[fo..fo + 64]
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect();
                    tracing::debug!("  Raw file bytes at 0x{:x}: {}", fo, preview.join(" "));
                }

                // Also dump bytes BEFORE the symbol to check if BundledAsset starts earlier
                // The symbol might point to something other than the struct start
                let asset_size = BundledAsset::MEMORY_LAYOUT.size();
                if fo >= asset_size {
                    let before_start = fo - asset_size;
                    let preview_before: Vec<String> = file_contents[before_start..before_start + 64]
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect();
                    tracing::debug!(
                        "  Raw file bytes {} BEFORE (0x{:x}): {}",
                        asset_size,
                        before_start,
                        preview_before.join(" ")
                    );

                    // Try to interpret as ASCII if it looks like a path
                    let ascii_preview: String = file_contents[before_start..before_start + 64]
                        .iter()
                        .map(|&b| if b >= 0x20 && b < 0x7f { b as char } else { '.' })
                        .collect();
                    tracing::debug!("  As ASCII: {:?}", ascii_preview);
                }
                break;
            }
            cumulative_offset += seg_size;
        }

        let Some(file_offset) = file_offset else {
            tracing::error!(
                "Virtual address 0x{:x} (data_relative: 0x{:x}) is beyond all data segments (total size: 0x{:x})",
                virtual_address,
                data_relative_offset,
                cumulative_offset
            );
            continue;
        };

        offsets.push(file_offset);
    }

    tracing::debug!("Found {} manganis symbol offsets", offsets.len());

    // Debug: Search for actual asset paths in the data section
    // This helps determine if asset data exists but at wrong offsets
    if !segment_file_info.is_empty() {
        let (first_seg_offset, first_seg_size) = segment_file_info[0];
        let seg_start = first_seg_offset as usize;
        let seg_end = (first_seg_offset + first_seg_size) as usize;

        if seg_end <= file_contents.len() {
            let segment_data = &file_contents[seg_start..seg_end];

            // Search for "/Users/" pattern which would indicate asset paths
            let search_pattern = b"/Users/";
            let mut found_paths = Vec::new();
            for (i, window) in segment_data.windows(search_pattern.len()).enumerate() {
                if window == search_pattern {
                    // Found a match - extract some context
                    let ctx_start = i;
                    let ctx_end = (i + 100).min(segment_data.len());
                    let context: String = segment_data[ctx_start..ctx_end]
                        .iter()
                        .take_while(|&&b| b != 0)
                        .map(|&b| if b >= 0x20 && b < 0x7f { b as char } else { '?' })
                        .collect();
                    found_paths.push((i, seg_start + i, context));
                    if found_paths.len() >= 10 {
                        break; // Limit output
                    }
                }
            }

            if !found_paths.is_empty() {
                tracing::debug!("--- Found '/Users/' paths in data segment ---");
                for (seg_offset, file_offset, context) in &found_paths {
                    tracing::debug!(
                        "  seg_offset=0x{:x}, file_offset=0x{:x}, virtual_addr=0x{:x}: {:?}",
                        seg_offset,
                        file_offset,
                        0x100000u64 + *seg_offset as u64,
                        context
                    );
                }
            } else {
                tracing::debug!("No '/Users/' paths found in data segment");
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
    let asset_size = BundledAsset::MEMORY_LAYOUT.size();
    tracing::debug!("BundledAsset::MEMORY_LAYOUT.size() = {} bytes", asset_size);

    for offset in offsets.iter().copied() {
        file.seek(std::io::SeekFrom::Start(offset))?;
        let mut data_in_range = vec![0; asset_size];
        file.read_exact(&mut data_in_range)?;

        // Debug: show structure of the data
        // BundledAsset layout: ConstStr (256 bytes + 4 len) + ConstStr (256 + 4) + AssetOptions
        // Show bytes around key boundaries
        tracing::debug!("Offset 0x{:x} ({}):", offset, offset);

        // First ConstStr bytes[0..32] - start of absolute_source_path string content
        let preview: Vec<String> = data_in_range[..32].iter().map(|b| format!("{:02x}", b)).collect();
        tracing::debug!("  bytes[0..32] (path start): {}", preview.join(" "));

        // First ConstStr len field at offset 256
        if data_in_range.len() > 260 {
            let len1 = u32::from_le_bytes([data_in_range[256], data_in_range[257], data_in_range[258], data_in_range[259]]);
            tracing::debug!("  bytes[256..260] (path1 len): {} (0x{:x})", len1, len1);

            // Show first 32 bytes of actual path content if len > 0
            if len1 > 0 && len1 < 256 {
                let path_preview: String = data_in_range[..len1.min(64) as usize]
                    .iter()
                    .filter(|&&b| b >= 0x20 && b < 0x7f)
                    .map(|&b| b as char)
                    .collect();
                tracing::debug!("  path1 content: {:?}", path_preview);
            }
        }

        // Second ConstStr len field at offset 260 + 256 = 516
        if data_in_range.len() > 520 {
            let len2 = u32::from_le_bytes([data_in_range[516], data_in_range[517], data_in_range[518], data_in_range[519]]);
            tracing::debug!("  bytes[516..520] (path2 len): {} (0x{:x})", len2, len2);
        }

        // AssetOptions starts at offset 520
        if data_in_range.len() > 520 {
            let options_preview: Vec<String> = data_in_range[520..std::cmp::min(544, data_in_range.len())]
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect();
            tracing::debug!("  bytes[520..544] (options): {}", options_preview.join(" "));
        }

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
