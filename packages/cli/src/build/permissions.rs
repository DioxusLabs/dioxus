//! The dioxus permission system.
//!
//! This module extracts permissions from compiled binaries and generates platform-specific
//! manifest files for platforms that require build-time permission declarations.
//!
//! Platforms requiring build-time manifests:
//! - Android: AndroidManifest.xml with <uses-permission> declarations
//! - iOS/macOS: Info.plist with usage description keys
//!
//! Other platforms (Linux, Web, Windows desktop) use runtime-only permissions
//! and do not require build-time manifest generation.

use std::{
    io::{Cursor, Read, Seek},
    path::{Path, PathBuf},
};

use crate::Result;
use anyhow::Context;
use const_serialize::SerializeConst;
use object::{File, Object, ObjectSection, ObjectSymbol, ReadCache, ReadRef, Section, Symbol};
use pdb::FallibleIterator;
use permissions_core::{Permission, Platform};
use serde::Serialize;

const PERMISSION_SYMBOL_PREFIX: &str = "__PERMISSION__";

/// Android permission for Handlebars template
#[derive(Debug, Clone, Serialize)]
pub struct AndroidPermission {
    pub name: String,
    pub description: String,
}

/// iOS permission for Handlebars template
#[derive(Debug, Clone, Serialize)]
pub struct IosPermission {
    pub key: String,
    pub description: String,
}

/// macOS permission for Handlebars template
#[derive(Debug, Clone, Serialize)]
pub struct MacosPermission {
    pub key: String,
    pub description: String,
}

/// Extract permission symbols from the object file
fn permission_symbols<'a, 'b, R: ReadRef<'a>>(
    file: &'b File<'a, R>,
) -> impl Iterator<Item = (Symbol<'a, 'b, R>, Section<'a, 'b, R>)> + 'b {
    file.symbols()
        .filter(|symbol| {
            if let Ok(name) = symbol.name() {
                name.contains(PERMISSION_SYMBOL_PREFIX)
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

fn looks_like_permission_symbol(name: &str) -> bool {
    name.contains(PERMISSION_SYMBOL_PREFIX)
}

/// Find the offsets of any permission symbols in the given file.
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
        // Otherwise, look for permission symbols in the object file.
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

/// Find the offsets of any permission symbols in a pdb file.
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
        if name.contains(PERMISSION_SYMBOL_PREFIX) {
            let section = sections
                .get(rva.section as usize - 1)
                .expect("Section index out of bounds");

            addresses.push((section.pointer_to_raw_data + rva.offset) as u64);
        }
    }
    Ok(addresses)
}

/// Find the offsets of any permission symbols in a native object file.
fn find_native_symbol_offsets<'a, R: ReadRef<'a>>(file: &File<'a, R>) -> Result<Vec<u64>> {
    let mut offsets = Vec::new();
    for (symbol, section) in permission_symbols(file) {
        let virtual_address = symbol.address();

        let Some((section_range_start, _)) = section.file_range() else {
            tracing::error!(
                "Found __PERMISSION__ symbol {:?} in section {}, but the section has no file range",
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

/// Find the offsets of any permission symbols in the wasm file.
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

    // Translate the section_relative_address to the file offset
    // WASM files have a section address of 0 in object, reparse the data section with wasmparser
    // to get the correct address and section start
    // Note: We need to reparse just the data section with wasmparser to get the file offset because walrus does
    // not expose the file offset information
    let reader = wasmparser::DataSectionReader::new(wasmparser::BinaryReader::new(
        &file_contents[section_start as usize..section_range_end as usize],
        0,
    ))
    .context("Failed to create WASM data section reader")?;
    let main_memory = reader
        .into_iter()
        .next()
        .context("Failed find main memory from WASM data section")?
        .context("Failed to read main memory from WASM data section")?;
    // main_memory.data is a slice somewhere in file_contents. Find out the offset in the file
    let data_start_offset = (main_memory.data.as_ptr() as u64)
        .checked_sub(file_contents.as_ptr() as u64)
        .expect("Data section start offset should be within the file contents");

    // Parse the wasm file to find the globals
    let module = walrus::Module::from_buffer(file_contents).unwrap();
    let mut offsets = Vec::new();

    // Find the main memory offset
    let main_memory = module
        .data
        .iter()
        .next()
        .context("Failed to find main memory in WASM module")?;

    let walrus::DataKind::Active {
        offset: main_memory_offset,
        ..
    } = main_memory.kind
    else {
        tracing::error!("Failed to find main memory offset in WASM module");
        return Ok(Vec::new());
    };

    // In the hot patch build, the main memory offset is a global from the main module and each global
    // is it's own global. Use an offset of 0 instead if we can't evaluate the global
    let main_memory_offset =
        eval_walrus_global_expr(&module, &main_memory_offset).unwrap_or_default();

    for export in module.exports.iter() {
        if !looks_like_permission_symbol(&export.name) {
            continue;
        }

        let walrus::ExportItem::Global(global) = export.item else {
            continue;
        };

        let walrus::GlobalKind::Local(pointer) = module.globals.get(global).kind else {
            continue;
        };

        let Some(virtual_address) = eval_walrus_global_expr(&module, &pointer) else {
            tracing::error!(
                "Found __PERMISSION__ symbol {:?} in WASM file, but the global expression could not be evaluated",
                export.name
            );
            continue;
        };

        let section_relative_address: u64 = ((virtual_address as i128)
            - main_memory_offset as i128)
            .try_into()
            .expect("Virtual address should be greater than or equal to section address");
        let file_offset = data_start_offset + section_relative_address;

        offsets.push(file_offset);
    }

    Ok(offsets)
}

/// Extract all permissions from the given file
pub(crate) fn extract_permissions_from_file(path: impl AsRef<Path>) -> Result<PermissionManifest> {
    let path = path.as_ref();
    let mut file = std::fs::File::open(path)?;

    let mut file_contents = Vec::new();
    file.read_to_end(&mut file_contents)?;
    let mut reader = Cursor::new(&file_contents);
    let read_cache = ReadCache::new(&mut reader);
    let object_file = object::File::parse(&read_cache)?;
    let offsets = find_symbol_offsets(path, &file_contents, &object_file)?;

    let mut permissions = Vec::new();

    for offset in offsets.iter().copied() {
        file.seek(std::io::SeekFrom::Start(offset))?;
        let mut data_in_range = vec![0; Permission::MEMORY_LAYOUT.size()];
        file.read_exact(&mut data_in_range)?;

        let buffer = const_serialize::ConstReadBuffer::new(&data_in_range);

        if let Some((_, permission)) = const_serialize::deserialize_const!(Permission, buffer) {
            tracing::debug!(
                "Found permission at offset {offset}: {:?}",
                permission.kind()
            );
            permissions.push(permission);
        } else {
            tracing::warn!(
                "Found permission symbol at offset {offset} that could not be deserialized"
            );
        }
    }

    Ok(PermissionManifest::new(permissions))
}

/// A manifest of all permissions found in a binary
#[derive(Debug, Clone, Default)]
pub struct PermissionManifest {
    permissions: Vec<Permission>,
}

impl PermissionManifest {
    pub fn new(permissions: Vec<Permission>) -> Self {
        Self { permissions }
    }

    pub fn permissions(&self) -> &[Permission] {
        &self.permissions
    }

    pub fn is_empty(&self) -> bool {
        self.permissions.is_empty()
    }

    pub fn permissions_for_platform(&self, platform: Platform) -> Vec<&Permission> {
        self.permissions
            .iter()
            .filter(|p| p.supports_platform(platform))
            .collect()
    }
}

/// Get Android permissions for Handlebars template
pub(crate) fn get_android_permissions(manifest: &PermissionManifest) -> Vec<AndroidPermission> {
    manifest
        .permissions_for_platform(Platform::Android)
        .iter()
        .filter_map(|perm| {
            perm.android_permission()
                .map(|android_perm| AndroidPermission {
                    name: android_perm.to_string(),
                    description: perm.description().to_string(),
                })
        })
        .collect()
}

/// Get iOS permissions for Handlebars template
pub(crate) fn get_ios_permissions(manifest: &PermissionManifest) -> Vec<IosPermission> {
    manifest
        .permissions_for_platform(Platform::Ios)
        .iter()
        .filter_map(|perm| {
            perm.ios_key().map(|key| IosPermission {
                key: key.to_string(),
                description: perm.description().to_string(),
            })
        })
        .collect()
}

/// Get macOS permissions for Handlebars template
pub(crate) fn get_macos_permissions(manifest: &PermissionManifest) -> Vec<MacosPermission> {
    manifest
        .permissions_for_platform(Platform::Macos)
        .iter()
        .filter_map(|perm| {
            perm.macos_key().map(|key| MacosPermission {
                key: key.to_string(),
                description: perm.description().to_string(),
            })
        })
        .collect()
}

/// Check if permissions are needed for the platform
pub(crate) fn needs_permission_manifest(platform: Platform) -> bool {
    matches!(
        platform,
        Platform::Android | Platform::Ios | Platform::Macos
    )
}
