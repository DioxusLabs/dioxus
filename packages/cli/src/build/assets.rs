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
//! symbol table to find symbols that match the `__ASSETS__` prefix. These symbols are ideally data
//! symbols and contain the BundledAsset data type which implements ConstSerialize and ConstDeserialize.
//!
//! When the binary is built, the `dioxus asset!()` macro will emit its metadata into the __ASSETS__
//! symbols, which we process here. After reading the metadata directly from the executable, we then
//! hash it and write the hash directly into the binary file.
//!
//! During development, we can skip this step for most platforms since local paths are sufficient
//! for asset loading. However, for WASM and for production builds, we need to ensure that assets
//! can be found relative to the current exe. Unfortunately, on android, the `current_exe` path is wrong,
//! so the assets are resolved against the "asset root" - which is covered by the asset loader crate.
//!
//! Finding the __ASSETS__ symbols is not quite straightforward when hotpatching, especially on WASM
//! since we build and link the module as relocatable, which is not a stable WASM proposal. In this
//! implementation, we handle both the non-PIE *and* PIC cases which are rather bespoke to our whole
//! build system.

use std::{
    io::{Cursor, Read, Seek, Write},
    path::{Path, PathBuf},
};

use crate::Result;
use anyhow::{bail, Context};
use const_serialize::{deserialize_const, serialize_const, ConstVec};
use dioxus_cli_opt::AssetManifest;
use manganis::{AssetOptions, AssetVariant, BundledAsset, ImageFormat, ImageSize};
use manganis_core::{AndroidArtifactMetadata, SwiftPackageMetadata, SymbolData};
use object::{File, Object, ObjectSection, ObjectSymbol, ReadCache, ReadRef, Section, Symbol};
use pdb::FallibleIterator;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};

/// Extract all manganis symbols and their sections from the given object file.
fn manganis_symbols<'a, 'b, R: ReadRef<'a>>(
    file: &'b File<'a, R>,
) -> impl Iterator<Item = (ManganisVersion, Symbol<'a, 'b, R>, Section<'a, 'b, R>)> + 'b {
    file.symbols().filter_map(move |symbol| {
        let name = symbol.name().ok()?;
        let version = looks_like_manganis_symbol(name)?;
        let section_index = symbol.section_index()?;
        let section = file.section_by_index(section_index).ok()?;
        Some((version, symbol, section))
    })
}

#[derive(Copy, Clone)]
enum ManganisVersion {
    /// The legacy version of the manganis format published with 0.7.0 and 0.7.1
    Legacy,
    /// The new version of the manganis format 0.7.2 onward
    /// This now includes both assets (old BundledAsset format) and permissions (SymbolData format)
    New,
}

impl ManganisVersion {
    fn size(&self) -> usize {
        match self {
            ManganisVersion::Legacy => {
                <manganis_core_07::BundledAsset as const_serialize_07::SerializeConst>::MEMORY_LAYOUT.size()
            }
            // For new format, we use a larger buffer size to accommodate variable-length CBOR
            // The actual size will be determined by CBOR deserialization
            ManganisVersion::New => 4096,
        }
    }

    /// Deserialize data, trying multiple formats for backward compatibility
    ///
    /// Tries in order:
    /// 1. SymbolData (new unified format) - can contain Asset or Permission
    /// 2. BundledAsset (old asset format) - for backward compatibility
    fn deserialize(&self, data: &[u8]) -> Option<SymbolDataOrAsset> {
        match self {
            ManganisVersion::Legacy => {
                let buffer = const_serialize_07::ConstReadBuffer::new(data);

                let (_, legacy_asset) =
                    const_serialize_07::deserialize_const!(manganis_core_07::BundledAsset, buffer)?;

                Some(SymbolDataOrAsset::Asset(legacy_asset_to_modern_asset(
                    &legacy_asset,
                )))
            }
            ManganisVersion::New => {
                // First try SymbolData (new format with enum variant)
                // const-serialize deserialization returns (remaining_bytes, value)
                // We accept if remaining is empty or contains only padding (zeros)
                if let Some((remaining, symbol_data)) = deserialize_const!(SymbolData, data) {
                    // Check if remaining bytes are all zeros (padding) or empty
                    // This handles the case where the linker section is larger than the actual data
                    // Be very lenient with padding - as long as we successfully deserialized, accept it
                    // The padding is just zeros added to fill the buffer size
                    let is_valid = remaining.is_empty()
                        || remaining.iter().all(|&b| b == 0)
                        || remaining.len() <= data.len(); // Allow any amount of padding as long as it's not larger than data

                    if is_valid {
                        return Some(SymbolDataOrAsset::SymbolData(Box::new(symbol_data)));
                    } else {
                        tracing::debug!(
                            "SymbolData deserialized but invalid padding: {} remaining bytes out of {} total (first few bytes: {:?})",
                            remaining.len(),
                            data.len(),
                            &data[..data.len().min(32)]
                        );
                    }
                } else {
                    tracing::debug!(
                        "Failed to deserialize as SymbolData. Data length: {}, first few bytes: {:?}",
                        data.len(),
                        &data[..data.len().min(32)]
                    );
                }

                // Fallback: try BundledAsset (direct format - assets are now serialized this way)
                // This handles assets that were serialized directly as BundledAsset (not wrapped in SymbolData)
                if let Some((remaining, asset)) = deserialize_const!(BundledAsset, data) {
                    // Check if remaining bytes are all zeros (padding) or empty
                    // Accept any amount of padding as long as it's all zeros (which is what we pad with)
                    let is_valid = remaining.is_empty() || remaining.iter().all(|&b| b == 0);

                    if is_valid {
                        tracing::debug!(
                            "Successfully deserialized BundledAsset, remaining padding: {} bytes",
                            remaining.len()
                        );
                        return Some(SymbolDataOrAsset::Asset(asset));
                    } else {
                        tracing::warn!(
                            "BundledAsset deserialized but remaining bytes are not all zeros: {} remaining bytes, first few: {:?}",
                            remaining.len(),
                            &remaining[..remaining.len().min(16)]
                        );
                    }
                } else {
                    tracing::warn!(
                        "Failed to deserialize as BundledAsset. Data length: {}, first 32 bytes: {:?}",
                        data.len(),
                        &data[..data.len().min(32)]
                    );
                }

                None
            }
        }
    }

    fn serialize_asset(&self, asset: &BundledAsset) -> Vec<u8> {
        match self {
            ManganisVersion::Legacy => {
                let legacy_asset = modern_asset_to_legacy_asset(asset);
                let buffer = const_serialize_07::serialize_const(
                    &legacy_asset,
                    const_serialize_07::ConstVec::new(),
                );
                buffer.as_ref().to_vec()
            }
            ManganisVersion::New => {
                // New format: serialize as BundledAsset directly (backward compatible)
                // Pad to 4096 bytes to match the linker output size
                let buffer = serialize_const(asset, ConstVec::new());
                let mut data = buffer.as_ref().to_vec();
                if data.len() < 4096 {
                    data.resize(4096, 0);
                }
                data
            }
        }
    }

    fn serialize_symbol_data(&self, data: &SymbolData) -> Option<Vec<u8>> {
        match self {
            ManganisVersion::Legacy => None,
            ManganisVersion::New => {
                let buffer = serialize_const(data, ConstVec::new());
                let mut bytes = buffer.as_ref().to_vec();
                if bytes.len() < 4096 {
                    bytes.resize(4096, 0);
                }
                Some(bytes)
            }
        }
    }
}

/// Result of deserializing a symbol - can be either SymbolData or legacy Asset
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
enum SymbolDataOrAsset {
    /// New unified format (can contain Asset or Permission)
    SymbolData(Box<SymbolData>),

    /// Old asset format (backward compatibility)
    Asset(BundledAsset),
}

#[derive(Clone, Copy)]
struct AssetWriteEntry {
    symbol: ManganisSymbolOffset,
    asset_index: usize,
    representation: AssetRepresentation,
}

impl AssetWriteEntry {
    fn new(
        symbol: ManganisSymbolOffset,
        asset_index: usize,
        representation: AssetRepresentation,
    ) -> Self {
        Self {
            symbol,
            asset_index,
            representation,
        }
    }
}

#[derive(Clone, Copy)]
enum AssetRepresentation {
    /// Serialized as a raw BundledAsset (legacy or new format)
    RawBundled,
    /// Serialized as SymbolData::Asset (new CBOR format)
    SymbolData,
}

fn legacy_asset_to_modern_asset(
    legacy_asset: &manganis_core_07::BundledAsset,
) -> manganis_core::BundledAsset {
    let bundled_path = legacy_asset.bundled_path();
    let absolute_path = legacy_asset.absolute_source_path();
    let legacy_options = legacy_asset.options();
    let add_hash = legacy_options.hash_suffix();
    let options = match legacy_options.variant() {
        manganis_core_07::AssetVariant::Image(image) => {
            let format = match image.format() {
                manganis_core_07::ImageFormat::Png => ImageFormat::Png,
                manganis_core_07::ImageFormat::Jpg => ImageFormat::Jpg,
                manganis_core_07::ImageFormat::Webp => ImageFormat::Webp,
                manganis_core_07::ImageFormat::Avif => ImageFormat::Avif,
                manganis_core_07::ImageFormat::Unknown => ImageFormat::Unknown,
            };
            let size = match image.size() {
                manganis_core_07::ImageSize::Automatic => ImageSize::Automatic,
                manganis_core_07::ImageSize::Manual { width, height } => {
                    ImageSize::Manual { width, height }
                }
            };
            let preload = image.preloaded();

            AssetOptions::image()
                .with_format(format)
                .with_size(size)
                .with_preload(preload)
                .with_hash_suffix(add_hash)
                .into_asset_options()
        }
        manganis_core_07::AssetVariant::Folder(_) => AssetOptions::folder()
            .with_hash_suffix(add_hash)
            .into_asset_options(),
        manganis_core_07::AssetVariant::Css(css) => AssetOptions::css()
            .with_hash_suffix(add_hash)
            .with_minify(css.minified())
            .with_preload(css.preloaded())
            .with_static_head(css.static_head())
            .into_asset_options(),
        manganis_core_07::AssetVariant::CssModule(css_module) => AssetOptions::css_module()
            .with_hash_suffix(add_hash)
            .with_minify(css_module.minified())
            .with_preload(css_module.preloaded())
            .into_asset_options(),
        manganis_core_07::AssetVariant::Js(js) => AssetOptions::js()
            .with_hash_suffix(add_hash)
            .with_minify(js.minified())
            .with_preload(js.preloaded())
            .with_static_head(js.static_head())
            .into_asset_options(),
        _ => AssetOptions::builder()
            .with_hash_suffix(add_hash)
            .into_asset_options(),
    };

    BundledAsset::new(absolute_path, bundled_path, options)
}

fn modern_asset_to_legacy_asset(modern_asset: &BundledAsset) -> manganis_core_07::BundledAsset {
    let bundled_path = modern_asset.bundled_path();
    let absolute_path = modern_asset.absolute_source_path();
    let legacy_options = modern_asset.options();
    let add_hash = legacy_options.hash_suffix();
    let options = match legacy_options.variant() {
        AssetVariant::Image(image) => {
            let format = match image.format() {
                ImageFormat::Png => manganis_core_07::ImageFormat::Png,
                ImageFormat::Jpg => manganis_core_07::ImageFormat::Jpg,
                ImageFormat::Webp => manganis_core_07::ImageFormat::Webp,
                ImageFormat::Avif => manganis_core_07::ImageFormat::Avif,
                ImageFormat::Unknown => manganis_core_07::ImageFormat::Unknown,
            };
            let size = match image.size() {
                ImageSize::Automatic => manganis_core_07::ImageSize::Automatic,
                ImageSize::Manual { width, height } => {
                    manganis_core_07::ImageSize::Manual { width, height }
                }
            };
            let preload = image.preloaded();

            manganis_core_07::AssetOptions::image()
                .with_format(format)
                .with_size(size)
                .with_preload(preload)
                .with_hash_suffix(add_hash)
                .into_asset_options()
        }
        AssetVariant::Folder(_) => manganis_core_07::AssetOptions::folder()
            .with_hash_suffix(add_hash)
            .into_asset_options(),
        AssetVariant::Css(css) => manganis_core_07::AssetOptions::css()
            .with_hash_suffix(add_hash)
            .with_minify(css.minified())
            .with_preload(css.preloaded())
            .with_static_head(css.static_head())
            .into_asset_options(),
        AssetVariant::CssModule(css_module) => manganis_core_07::AssetOptions::css_module()
            .with_hash_suffix(add_hash)
            .with_minify(css_module.minified())
            .with_preload(css_module.preloaded())
            .into_asset_options(),
        AssetVariant::Js(js) => manganis_core_07::AssetOptions::js()
            .with_hash_suffix(add_hash)
            .with_minify(js.minified())
            .with_preload(js.preloaded())
            .with_static_head(js.static_head())
            .into_asset_options(),
        _ => manganis_core_07::AssetOptions::builder()
            .with_hash_suffix(add_hash)
            .into_asset_options(),
    };

    manganis_core_07::BundledAsset::new(absolute_path, bundled_path, options)
}

fn looks_like_manganis_symbol(name: &str) -> Option<ManganisVersion> {
    if name.contains("__MANGANIS__") {
        Some(ManganisVersion::Legacy)
    } else if name.contains("__ASSETS__") {
        Some(ManganisVersion::New)
    } else {
        None
    }
}

/// An asset offset in the binary
#[derive(Clone, Copy)]
struct ManganisSymbolOffset {
    version: ManganisVersion,
    offset: u64,
}

impl ManganisSymbolOffset {
    fn new(version: ManganisVersion, offset: u64) -> Self {
        Self { version, offset }
    }
}

/// Find the offsets of any manganis symbols in the given file.
fn find_symbol_offsets<'a, R: ReadRef<'a>>(
    path: &Path,
    file_contents: &[u8],
    file: &File<'a, R>,
) -> Result<Vec<ManganisSymbolOffset>> {
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
fn find_pdb_symbol_offsets(pdb_file: &Path) -> Result<Vec<ManganisSymbolOffset>> {
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
        if let Some(version) = looks_like_manganis_symbol(&name) {
            let section = sections
                .get(rva.section as usize - 1)
                .expect("Section index out of bounds");

            addresses.push(ManganisSymbolOffset::new(
                version,
                (section.pointer_to_raw_data + rva.offset) as u64,
            ));
        }
    }
    Ok(addresses)
}

/// Find the offsets of any manganis symbols in a native object file.
fn find_native_symbol_offsets<'a, R: ReadRef<'a>>(
    file: &File<'a, R>,
) -> Result<Vec<ManganisSymbolOffset>> {
    let mut offsets = Vec::new();
    for (version, symbol, section) in manganis_symbols(file) {
        let virtual_address = symbol.address();

        let Some((section_range_start, _)) = section.file_range() else {
            tracing::error!(
                "Found __ASSETS__ symbol {:?} in section {}, but the section has no file range",
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
        offsets.push(ManganisSymbolOffset::new(version, file_offset));
    }

    Ok(offsets)
}

/// Evaluate a walrus global expression to get its value.
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

/// Find the value of a global export by name.
fn find_global_export_value(module: &walrus::Module, name: &str) -> Option<u64> {
    for export in module.exports.iter() {
        if export.name == name {
            if let walrus::ExportItem::Global(g) = export.item {
                if let walrus::GlobalKind::Local(expr) = &module.globals.get(g).kind {
                    return eval_walrus_global_expr(module, expr);
                }
            }
        }
    }
    None
}

/// Find the offsets of any manganis symbols in the wasm file.
///
/// This handles both standard WASM builds and builds with advanced features like:
/// - Bulk memory operations (passive data segments)
/// - Thread Local Storage (TLS)
/// - Atomics and shared memory
fn find_wasm_symbol_offsets<'a, R: ReadRef<'a>>(
    file_contents: &[u8],
    file: &File<'a, R>,
) -> Result<Vec<ManganisSymbolOffset>> {
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

    // Parse data segments with wasmparser to get file offsets.
    // Walrus doesn't expose file offset information, so we need wasmparser for this.
    // With bulk memory operations, there may be multiple data segments.
    let reader = wasmparser::DataSectionReader::new(wasmparser::BinaryReader::new(
        &file_contents[section_start as usize..section_range_end as usize],
        0,
    ))
    .context("Failed to create WASM data section reader")?;

    // Collect all data segments with their file offsets and sizes
    let mut segment_file_info: Vec<(u64, u64)> = Vec::new();
    for segment in reader.into_iter() {
        let segment = segment.context("Failed to read data segment")?;
        segment_file_info.push((
            (segment.data.as_ptr() as u64)
                .checked_sub(file_contents.as_ptr() as u64)
                .expect("Data segment should be within file contents"),
            segment.data.len() as u64,
        ));
    }

    if segment_file_info.is_empty() {
        return Ok(Vec::new());
    }

    // Parse the wasm file with walrus to find globals and exports
    let module = walrus::Module::from_buffer(file_contents)
        .context("Failed to parse WASM module with walrus")?;

    // Determine the memory base address for symbol lookup
    let main_memory_walrus = module
        .data
        .iter()
        .next()
        .context("Failed to find main memory in WASM module")?;

    let main_memory_offset = match &main_memory_walrus.kind {
        walrus::DataKind::Active { offset, .. } => {
            // Active segments have an explicit offset expression
            eval_walrus_global_expr(&module, offset).unwrap_or_default()
        }
        walrus::DataKind::Passive => {
            // For passive segments (bulk memory operations), there's no static offset.
            // The memory.init instruction determines placement at runtime.
            //
            // Try to find the actual memory base from linker exports:
            // - __memory_base: Set by the linker for bulk-memory builds
            // - Falls back to 0x100000 (Rust/LLVM default for static data)
            //
            // With TLS support, the linker calculates symbol addresses as if TLS data
            // is at the base address followed by main data. But at runtime, TLS is stored
            // separately per-thread via __wasm_init_tls. We detect TLS by looking for
            // __tls_size and adjust accordingly.
            //
            // IMPORTANT: The linker aligns main data to a 4-byte boundary after TLS.
            // This alignment padding exists in MEMORY but NOT in the FILE. We must
            // use the aligned TLS size for base calculation, but the file segments
            // are stored without this padding.
            let memory_base = find_global_export_value(&module, "__memory_base");
            let tls_size = find_global_export_value(&module, "__tls_size").unwrap_or(0);

            // If TLS is present and segment 0 matches TLS size, remove TLS segment
            // from our file info since it's not where data symbols point
            if tls_size > 0 && !segment_file_info.is_empty() && segment_file_info[0].1 == tls_size {
                segment_file_info.remove(0);
            }

            // Align TLS size up to 4 bytes to match linker's memory layout.
            // The linker aligns main data to a 4-byte boundary after TLS, so symbol
            // addresses are calculated from (memory_base + aligned_tls_size).
            // However, file segments are stored without this alignment padding.
            let tls_aligned = (tls_size + 3) & !3;

            // Use __memory_base if available (set by linker in release builds),
            // otherwise fall back to 0x100000 (debug builds default)
            memory_base.unwrap_or(0x100000u64) + tls_aligned
        }
    };

    // Find all manganis symbols and calculate their file offsets
    let mut offsets = Vec::new();

    for export in module.exports.iter() {
        let Some(version) = looks_like_manganis_symbol(&export.name) else {
            continue;
        };

        let walrus::ExportItem::Global(global) = export.item else {
            continue;
        };

        let global_data = module.globals.get(global);
        let walrus::GlobalKind::Local(pointer) = global_data.kind else {
            continue;
        };

        let Some(virtual_address) = eval_walrus_global_expr(&module, &pointer) else {
            tracing::error!(
                "Found __ASSETS__ symbol {:?} in WASM file, but the global expression could not be evaluated",
                export.name
            );
            continue;
        };

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

        // Find which segment this offset falls into.
        // Segments are laid out contiguously in memory.
        let mut cumulative_offset = 0u64;
        let mut file_offset = None;

        for (seg_file_offset, seg_size) in segment_file_info.iter() {
            if data_relative_offset < cumulative_offset + seg_size {
                let offset_in_segment = data_relative_offset - cumulative_offset;
                file_offset = Some(seg_file_offset + offset_in_segment);
                break;
            }
            cumulative_offset += seg_size;
        }

        let Some(file_offset) = file_offset else {
            tracing::error!(
                "Virtual address 0x{:x} is beyond all data segments",
                virtual_address
            );
            continue;
        };

        offsets.push(ManganisSymbolOffset::new(version, file_offset));
    }

    Ok(offsets)
}

/// Result of extracting symbols from a binary file
#[derive(Debug, Clone)]
pub(crate) struct SymbolExtractionResult {
    /// Assets found in the binary
    pub assets: Vec<BundledAsset>,

    /// Android plugin artifacts discovered in the binary
    pub android_artifacts: Vec<AndroidArtifactMetadata>,

    /// Swift packages discovered in the binary
    pub swift_packages: Vec<SwiftPackageMetadata>,
}

/// Find all assets in the given file, hash them, and write them back to the file.
/// Also extracts Android/Swift plugin metadata for FFI bindings.
pub(crate) async fn extract_symbols_from_file(
    path: impl AsRef<Path>,
) -> Result<SymbolExtractionResult> {
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
    let mut android_artifacts = Vec::new();
    let mut swift_packages = Vec::new();
    let mut write_entries = Vec::new();

    // Read each symbol from the data section using the offsets
    for symbol in offsets.iter().copied() {
        let version = symbol.version;
        let offset = symbol.offset;

        // Read data from file_contents (already loaded into memory)
        // Use a large buffer for variable length data, but don't exceed file size
        let buffer_size = version
            .size()
            .min(file_contents.len().saturating_sub(offset as usize));
        if buffer_size == 0 {
            tracing::warn!("Symbol at offset {offset} is beyond file size");
            continue;
        }

        let data_in_range = if (offset as usize) + buffer_size <= file_contents.len() {
            &file_contents[offset as usize..(offset as usize) + buffer_size]
        } else {
            &file_contents[offset as usize..]
        };

        // Try to deserialize - const-serialize will handle variable-length data correctly
        // The deserialization should work even with padding (zeros) at the end
        if let Some(result) = version.deserialize(data_in_range) {
            match result {
                SymbolDataOrAsset::SymbolData(symbol_data) => match *symbol_data {
                    SymbolData::Asset(asset) => {
                        tracing::debug!(
                            "Found asset (via SymbolData) at offset {offset}: {:?}",
                            asset.absolute_source_path()
                        );
                        let asset_index = assets.len();
                        assets.push(asset);
                        write_entries.push(AssetWriteEntry::new(
                            symbol,
                            asset_index,
                            AssetRepresentation::SymbolData,
                        ));
                    }

                    SymbolData::AndroidArtifact(meta) => {
                        tracing::debug!(
                            "Found Android artifact declaration for plugin {}",
                            meta.plugin_name.as_str()
                        );
                        android_artifacts.push(meta);
                    }
                    SymbolData::SwiftPackage(meta) => {
                        tracing::debug!(
                            "Found Swift package declaration for plugin {}",
                            meta.plugin_name.as_str()
                        );
                        swift_packages.push(meta);
                    }
                    _ => {}
                },
                SymbolDataOrAsset::Asset(asset) => {
                    tracing::debug!(
                        "Found asset (old format) at offset {offset}: {:?}",
                        asset.absolute_source_path()
                    );
                    let asset_index = assets.len();
                    assets.push(asset);
                    write_entries.push(AssetWriteEntry::new(
                        symbol,
                        asset_index,
                        AssetRepresentation::RawBundled,
                    ));
                }
            }
        } else {
            tracing::warn!("Found a symbol at offset {offset} that could not be deserialized. This may be caused by a mismatch between your dioxus and dioxus-cli versions, or the symbol may be in an unsupported format.");
        }
    }

    // Add the hash to each asset in parallel
    assets
        .par_iter_mut()
        .for_each(dioxus_cli_opt::add_hash_to_asset);

    // Write back only assets to the binary file (permissions are not modified)
    for entry in write_entries {
        let version = entry.symbol.version;
        let offset = entry.symbol.offset;
        let asset = assets
            .get(entry.asset_index)
            .copied()
            .expect("asset index collected from symbol scan");

        match entry.representation {
            AssetRepresentation::RawBundled => {
                tracing::debug!("Writing asset to offset {offset}: {:?}", asset);
                let new_data = version.serialize_asset(&asset);
                if new_data.len() > version.size() {
                    tracing::warn!(
                        "Asset at offset {offset} serialized to {} bytes, but buffer is only {} bytes. Truncating output.",
                        new_data.len(),
                        version.size()
                    );
                }
                write_serialized_bytes(&mut file, offset, &new_data, version.size())?;
            }
            AssetRepresentation::SymbolData => {
                tracing::debug!("Writing asset (SymbolData) to offset {offset}: {:?}", asset);
                let Some(new_data) = version.serialize_symbol_data(&SymbolData::Asset(asset))
                else {
                    tracing::warn!(
                        "Symbol at offset {offset} was stored as SymbolData but the binary format only supports raw assets"
                    );
                    continue;
                };
                if new_data.len() > version.size() {
                    tracing::warn!(
                        "SymbolData asset at offset {offset} serialized to {} bytes, but buffer is only {} bytes. Truncating output.",
                        new_data.len(),
                        version.size()
                    );
                }
                write_serialized_bytes(&mut file, offset, &new_data, version.size())?;
            }
        }
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

    Ok(SymbolExtractionResult {
        assets,
        android_artifacts,
        swift_packages,
    })
}

/// Find all assets in the given file, hash them, and write them back to the file.
/// Then return an `AssetManifest` containing all the assets found in the file.
///
/// This is a convenience function that extracts symbols and returns only assets.
pub(crate) async fn extract_assets_from_file(path: impl AsRef<Path>) -> Result<AssetManifest> {
    let result = extract_symbols_from_file(path).await?;
    let mut manifest = AssetManifest::default();
    for asset in result.assets {
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

fn write_serialized_bytes(
    file: &mut std::fs::File,
    offset: u64,
    data: &[u8],
    buffer_size: usize,
) -> Result<()> {
    use std::io::SeekFrom;

    file.seek(SeekFrom::Start(offset))?;
    if data.len() <= buffer_size {
        file.write_all(data)?;
        if data.len() < buffer_size {
            let padding = vec![0; buffer_size - data.len()];
            file.write_all(&padding)?;
        }
    } else {
        file.write_all(&data[..buffer_size])?;
    }

    Ok(())
}
