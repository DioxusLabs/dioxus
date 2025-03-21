use anyhow::{Context, Result};
use itertools::Itertools;
use memmap::{Mmap, MmapOptions};
use object::{
    macho,
    read::File,
    write::{MachOBuildVersion, Symbol, SymbolSection},
    Architecture, BinaryFormat, Endianness, Object, ObjectSection, ObjectSymbol, ObjectSymbolTable,
    Relocation, RelocationTarget, SectionIndex, SectionKind, SymbolKind, SymbolScope,
};
use std::io::Write;
use std::{cmp::Ordering, ffi::OsStr, fs, ops::Deref, path::PathBuf};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
};
pub use subsecond_types::*;
use target_lexicon::{OperatingSystem, Triple};
use tokio::process::Command;

/// Resolve the undefined symbols in the incrementals against the original binary, returning an object
/// file that can be linked along the incrementals.
///
/// This makes it possible to dlopen the resulting object file and use the original binary's symbols
/// bypassing the dynamic linker.
///
/// This is very similar to malware :) but it's not!
pub fn resolve_undefined(
    source: &Path,
    incrementals: &[PathBuf],
    triple: &Triple,
    aslr_reference: u64,
) -> Result<Vec<u8>> {
    let sorted: Vec<_> = incrementals.iter().sorted().collect();

    // Find all the undefined symbols in the incrementals
    let mut undefined_symbols = HashSet::new();
    let mut defined_symbols = HashSet::new();
    for path in sorted {
        let bytes = std::fs::read(&path)?;
        let file = File::parse(bytes.deref() as &[u8])?;
        for symbol in file.symbols() {
            if symbol.is_undefined() {
                undefined_symbols.insert(symbol.name()?.to_string());
            } else {
                if symbol.is_global() {
                    defined_symbols.insert(symbol.name()?.to_string());
                }
            }
        }
    }
    let undefined_symbols: Vec<_> = undefined_symbols
        .difference(&defined_symbols)
        .cloned()
        .collect();

    // Create a new object file (architecture doesn't matter much for our purposes)
    let mut obj = object::write::Object::new(
        match triple.binary_format {
            target_lexicon::BinaryFormat::Elf => object::BinaryFormat::Elf,
            target_lexicon::BinaryFormat::Macho => object::BinaryFormat::MachO,
            target_lexicon::BinaryFormat::Coff => todo!(),
            target_lexicon::BinaryFormat::Wasm => todo!(),
            target_lexicon::BinaryFormat::Xcoff => todo!(),
            _ => todo!(),
        },
        match triple.architecture {
            target_lexicon::Architecture::Aarch64(_) => object::Architecture::Aarch64,
            _ => todo!(),
        },
        match triple.endianness().unwrap() {
            target_lexicon::Endianness::Little => Endianness::Little,
            target_lexicon::Endianness::Big => Endianness::Big,
        },
    );

    match triple.operating_system {
        target_lexicon::OperatingSystem::Darwin(_) => {
            obj.set_macho_build_version({
                let mut build_version = MachOBuildVersion::default();
                build_version.platform = macho::PLATFORM_MACOS;
                build_version.minos = (11 << 16) | (0 << 8) | 0;
                build_version.sdk = (11 << 16) | (0 << 8) | 0;
                build_version
            });
        }
        target_lexicon::OperatingSystem::IOS(_) => {
            obj.set_macho_build_version({
                let mut build_version = MachOBuildVersion::default();
                build_version.platform = macho::PLATFORM_IOS;
                build_version.minos = (14 << 16) | (0 << 8) | 0; // 14.0.0
                build_version.sdk = (14 << 16) | (0 << 8) | 0; // SDK 14.0.0
                build_version
            });
        }

        _ => {}
    }

    // Load the original binary
    let bytes = std::fs::read(&source)?;
    let file = File::parse(bytes.deref() as &[u8])?;
    let symbol_table = file
        .symbols()
        .flat_map(|s| Some((s.name().ok()?, s)))
        .collect::<HashMap<_, _>>();
    let aslr_offset = aslr_reference - symbol_table.get("_aslr_reference").unwrap().address();

    for name in undefined_symbols {
        if let Some(sym) = symbol_table.get(name.as_str()) {
            let address = sym.address() + aslr_offset;

            obj.add_symbol(Symbol {
                name: name.as_bytes()[1..].to_vec(),
                value: address,
                size: 0,
                scope: SymbolScope::Dynamic,
                weak: sym.is_weak(),
                kind: sym.kind(),
                section: SymbolSection::Absolute,
                flags: object::SymbolFlags::None,
            });
        } else {
            println!("Symbol not found: {}", name);
        }
    }

    // Write the object to a file
    let bytes = obj.write()?;
    Ok(bytes)
}

#[test]
fn test_resolve_undefined() {
    let incremental_dir = "/Users/jonkelley/Development/dioxus/packages/subsecond/data/linux/rcgu";
    let incrementals = std::fs::read_dir(incremental_dir)
        .unwrap()
        .map(|x| x.unwrap().path())
        .collect::<Vec<_>>();

    let source_file: PathBuf =
        "/Users/jonkelley/Development/dioxus/packages/subsecond/data/linux/subsecond-harness"
            .into();

    let bytes = resolve_undefined(
        &source_file,
        &incrementals,
        "aarch64-android-linux".parse().unwrap(),
        0,
    )
    .unwrap();
}
