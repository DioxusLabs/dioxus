//! Utilities for extracting metadata from linker sections
//!
//! This module provides generic utilities for extracting metadata embedded in compiled binaries
//! via linker sections. It's used by the asset/permission collector, Android plugin artifact
//! discovery, and the Swift metadata scanner.

use std::path::Path;

use crate::Result;
use object::{File, Object, ObjectSection, ObjectSymbol, ReadCache, ReadRef, Section, Symbol};

/// Extract symbols from an object file that match a given prefix
///
/// This is a generic utility used across metadata collectors (assets/permissions, Android artifacts,
/// Swift sources, etc).
pub fn extract_symbols_with_prefix<'a, 'b, R: ReadRef<'a>>(
    file: &'b File<'a, R>,
    prefix: &'b str,
) -> impl Iterator<Item = (Symbol<'a, 'b, R>, Section<'a, 'b, R>)> + 'b {
    let prefix = prefix.to_string(); // Clone to avoid lifetime issues
    file.symbols()
        .filter(move |symbol| {
            if let Ok(name) = symbol.name() {
                name.contains(&prefix)
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

/// Find the file offsets of symbols matching the given prefix
///
/// This function handles native object files (ELF/Mach-O) which are used for
/// Android, iOS, and macOS builds.
pub fn find_symbol_offsets_from_object<'a, R: ReadRef<'a>>(
    file: &File<'a, R>,
    prefix: &str,
) -> Result<Vec<u64>> {
    let mut offsets = Vec::new();

    for (symbol, section) in extract_symbols_with_prefix(file, prefix) {
        let virtual_address = symbol.address();

        let Some((section_range_start, _)) = section.file_range() else {
            tracing::error!(
                "Found {} symbol {:?} in section {}, but the section has no file range",
                prefix,
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

/// Find symbol offsets from a file path
///
/// Opens the file, parses it as an object file, and returns the offsets.
pub fn find_symbol_offsets_from_path(path: &Path, prefix: &str) -> Result<Vec<u64>> {
    let mut file = std::fs::File::open(path)?;
    let mut file_contents = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut file_contents)?;

    let mut reader = std::io::Cursor::new(&file_contents);
    let read_cache = ReadCache::new(&mut reader);
    let object_file = object::File::parse(&read_cache)?;

    find_symbol_offsets_from_object(&object_file, prefix)
}
