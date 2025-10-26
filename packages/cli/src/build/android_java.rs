//! Android Java source collection from compiled binaries
//!
//! This module extracts Java source metadata from embedded linker symbols,
//! similar to how permissions and manganis work. It finds `__JAVA_SOURCE__`
//! symbols in the binary and deserializes them into metadata that can be
//! used by the Gradle build process.

use std::io::Read;
use std::path::Path;

use crate::Result;
use anyhow::Context;
use object::{File, Object, ObjectSection, ObjectSymbol, ReadCache, ReadRef, Section, Symbol};
use pdb::FallibleIterator;

const JAVA_SOURCE_SYMBOL_PREFIX: &str = "__JAVA_SOURCE__";

/// Extract Java source symbols from the object file
fn java_source_symbols<'a, 'b, R: ReadRef<'a>>(
    file: &'b File<'a, R>,
) -> impl Iterator<Item = (Symbol<'a, 'b, R>, Section<'a, 'b, R>)> + 'b {
    file.symbols()
        .filter(|symbol| {
            if let Ok(name) = symbol.name() {
                name.contains(JAVA_SOURCE_SYMBOL_PREFIX)
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

fn looks_like_java_source_symbol(name: &str) -> bool {
    name.contains(JAVA_SOURCE_SYMBOL_PREFIX)
}

/// Find the offsets of any Java source symbols in the given file
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
        object::BinaryFormat::Pe if pdb_file.is_some() => {
            find_pdb_symbol_offsets(&pdb_file.unwrap())
        }
        // Otherwise, look for Java source symbols in the object file.
        _ => find_native_symbol_offsets(file),
    }
}

/// Find the pdb file matching the executable file
fn find_pdb_file(path: &Path) -> Option<std::path::PathBuf> {
    let mut pdb_file = path.with_extension("pdb");
    if let Some(file_name) = pdb_file.file_name() {
        let new_file_name = file_name.to_string_lossy().replace('-', "_");
        let altrnate_pdb_file = pdb_file.with_file_name(new_file_name);
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
            (Err(_), Ok(_)) => pdb_file = altrnate_pdb_file,
            _ => {}
        }
    }
    if pdb_file.exists() {
        Some(pdb_file)
    } else {
        None
    }
}

/// Find the offsets of any Java source symbols in a pdb file
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
        if name.contains(JAVA_SOURCE_SYMBOL_PREFIX) {
            let section = sections
                .get(rva.section as usize - 1)
                .expect("Section index out of bounds");

            addresses.push((section.pointer_to_raw_data + rva.offset) as u64);
        }
    }
    Ok(addresses)
}

/// Find the offsets of any Java source symbols in a native object file
fn find_native_symbol_offsets<'a, R: ReadRef<'a>>(file: &File<'a, R>) -> Result<Vec<u64>> {
    let mut offsets = Vec::new();
    for (symbol, section) in java_source_symbols(file) {
        let virtual_address = symbol.address();

        let Some((section_range_start, _)) = section.file_range() else {
            tracing::error!(
                "Found __JAVA_SOURCE__ symbol {:?} in section {}, but the section has no file range",
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

/// Find the offsets of any Java source symbols in the wasm file
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

    // Evaluate the global expression if possible
    let main_memory_offset =
        eval_walrus_global_expr(&module, &main_memory_offset).unwrap_or_default();

    for export in module.exports.iter() {
        if !looks_like_java_source_symbol(&export.name) {
            continue;
        }

        let walrus::ExportItem::Global(global) = export.item else {
            continue;
        };

        let walrus::GlobalKind::Local(pointer) = module.globals.get(global).kind else {
            continue;
        };

        let Some(virtual_address) = eval_walrus_global_expr(&module, &pointer) else {
            continue;
        };

        let section_relative_address: u64 = ((virtual_address as i128)
            - main_memory_offset as i128)
            .try_into()
            .expect("Virtual address should be greater than or equal to section address");
        let file_offset = section_start + section_relative_address;

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

/// Metadata about Java sources that need to be compiled to DEX
/// This mirrors the struct from mobile-core
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaSourceMetadata {
    /// File paths relative to crate root
    pub files: Vec<String>,
    /// Java package name (e.g. "dioxus.mobile.geolocation")
    pub package_name: String,
    /// Plugin identifier for organization (e.g. "geolocation")
    pub plugin_name: String,
}

impl JavaSourceMetadata {
    /// Create from the mobile-core SerializeConst version
    fn from_const_serialize(
        package_name: const_serialize::ConstStr,
        plugin_name: const_serialize::ConstStr,
        file_count: u8,
        files: [const_serialize::ConstStr; 8],
    ) -> Self {
        Self {
            package_name: package_name.as_str().to_string(),
            plugin_name: plugin_name.as_str().to_string(),
            files: files[..file_count as usize]
                .iter()
                .map(|s| s.as_str().to_string())
                .collect(),
        }
    }
}

/// A manifest of all Java sources found in a binary
#[derive(Debug, Clone, Default)]
pub struct JavaSourceManifest {
    sources: Vec<JavaSourceMetadata>,
}

impl JavaSourceManifest {
    pub fn new(sources: Vec<JavaSourceMetadata>) -> Self {
        Self { sources }
    }

    pub fn sources(&self) -> &[JavaSourceMetadata] {
        &self.sources
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

/// Extract all Java sources from the given file
pub(crate) fn extract_java_sources_from_file(path: impl AsRef<Path>) -> Result<JavaSourceManifest> {
    let path = path.as_ref();
    let mut file = std::fs::File::open(path)?;

    let mut file_contents = Vec::new();
    file.read_to_end(&mut file_contents)?;
    let mut reader = std::io::Cursor::new(&file_contents);
    let read_cache = ReadCache::new(&mut reader);
    let object_file = object::File::parse(&read_cache)?;
    let offsets = find_symbol_offsets(path, &file_contents, &object_file)?;

    let mut sources = Vec::new();

    // Parse the metadata from each symbol offset
    // The format is: (package_name: &str, plugin_name: &str, files: &[&str])
    for offset in offsets {
        match parse_java_metadata_at_offset(&file_contents, offset as usize) {
            Ok(metadata) => {
                tracing::debug!(
                    "Extracted Java metadata: plugin={}, package={}, files={:?}",
                    metadata.plugin_name,
                    metadata.package_name,
                    metadata.files
                );
                sources.push(metadata);
            }
            Err(e) => {
                tracing::warn!("Failed to parse Java metadata at offset {}: {}", offset, e);
            }
        }
    }

    if !sources.is_empty() {
        tracing::info!(
            "Extracted {} Java source declarations from binary",
            sources.len()
        );
    }

    Ok(JavaSourceManifest::new(sources))
}

/// Parse Java metadata from binary data at the given offset
///
/// The data is serialized using const-serialize and contains:
/// - package_name: ConstStr
/// - plugin_name: ConstStr  
/// - file_count: u8
/// - files: [ConstStr; 8]
fn parse_java_metadata_at_offset(data: &[u8], offset: usize) -> Result<JavaSourceMetadata> {
    use const_serialize::ConstStr;

    // Read the serialized data (padded to 4096 bytes like permissions)
    let end = (offset + 4096).min(data.len());
    let metadata_bytes = &data[offset..end];

    let buffer = const_serialize::ConstReadBuffer::new(metadata_bytes);

    // Deserialize the struct fields
    // The SerializeConst derive creates a tuple-like serialization
    if let Some((buffer, package_name)) = const_serialize::deserialize_const!(ConstStr, buffer) {
        if let Some((buffer, plugin_name)) = const_serialize::deserialize_const!(ConstStr, buffer) {
            if let Some((buffer, file_count)) = const_serialize::deserialize_const!(u8, buffer) {
                if let Some((_, files)) = const_serialize::deserialize_const!([ConstStr; 8], buffer)
                {
                    return Ok(JavaSourceMetadata::from_const_serialize(
                        package_name,
                        plugin_name,
                        file_count,
                        files,
                    ));
                }
            }
        }
    }

    anyhow::bail!("Failed to deserialize Java metadata at offset {}", offset)
}
