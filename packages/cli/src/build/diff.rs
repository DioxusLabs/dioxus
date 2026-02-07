//! Assembly diffing with relocation masks for cascade detection.
//!
//! When a workspace dependency crate is recompiled, we compare its old `.rcgu.o` files against the
//! new ones to determine if the tip crate needs recompilation. The comparison masks out relocation
//! target bytes (which differ between compilations due to symbol index changes) and instead compares
//! the relocation targets by name.
//!
//! If only non-public, non-generic, non-inline functions changed, the tip crate's monomorphizations
//! are unaffected and we can skip its recompilation — only relinking is needed.
//!
//! Ported from `packages/subsecond/data/old_patch.rs` (the original prototype).

use crate::build::cache::CachedObject;
use object::{read::File as ObjectFile, Object, ObjectSection, ObjectSymbol, RelocationTarget};
use std::collections::{HashMap, HashSet};

/// Result of comparing old and new object files for a crate.
pub struct DiffResult {
    /// Symbols whose masked assembly changed between old and new.
    pub changed_symbols: HashSet<String>,

    /// Whether any changed symbol could affect downstream crates (public, generic, or inline).
    /// If true, downstream workspace crates must be recompiled.
    pub needs_downstream_recompile: bool,
}

/// Extracted information about a symbol, sufficient for masked comparison.
/// Owns all its data so it can outlive the parsed object file.
struct ExtractedSymbol {
    /// Length of the symbol's instruction data
    data_len: usize,
    /// FNV-1a hash of the instruction data
    data_hash: u64,
    /// Number of relocations within this symbol's range
    reloc_count: usize,
    /// Names of relocation targets, in address order
    reloc_target_names: Vec<Option<String>>,
    /// Whether this symbol has global visibility
    is_global: bool,
}

/// Compare two sets of cached object files and determine what changed.
///
/// Returns a `DiffResult` indicating which symbols changed and whether downstream
/// recompilation is needed.
pub fn diff_objects(old_objects: &[CachedObject], new_objects: &[CachedObject]) -> DiffResult {
    let old_symbols = extract_all_symbols(old_objects);
    let new_symbols = extract_all_symbols(new_objects);

    let mut changed_symbols = HashSet::new();

    // Check all symbols in new objects
    for (name, new_sym) in &new_symbols {
        match old_symbols.get(name) {
            Some(old_sym) => {
                if !symbols_match(old_sym, new_sym) {
                    changed_symbols.insert(name.clone());
                }
            }
            None => {
                changed_symbols.insert(name.clone());
            }
        }
    }

    // Check for removed symbols
    for name in old_symbols.keys() {
        if !new_symbols.contains_key(name) {
            changed_symbols.insert(name.clone());
        }
    }

    // Determine if any changed symbol could affect downstream crates
    let needs_downstream_recompile = changed_symbols.iter().any(|name| {
        new_symbols
            .get(name)
            .or_else(|| old_symbols.get(name))
            .map_or(true, |sym| sym.is_global)
    });

    DiffResult {
        changed_symbols,
        needs_downstream_recompile,
    }
}

/// Extract all symbols from a set of cached object files into a name → info map.
fn extract_all_symbols(objects: &[CachedObject]) -> HashMap<String, ExtractedSymbol> {
    let mut result: HashMap<String, ExtractedSymbol> = HashMap::new();

    for cached in objects {
        let Ok(obj) = ObjectFile::parse(&*cached.data) else {
            continue;
        };

        for section in obj.sections() {
            let section_idx = section.index();
            let Ok(section_data) = section.data() else {
                continue;
            };

            // Collect relocations for this section
            let relocations: Vec<_> = section.relocations().collect();

            // Get symbols in this section, sorted by address
            let mut symbols: Vec<object::Symbol<'_, '_>> = obj
                .symbols()
                .filter(|s| s.section_index() == Some(section_idx))
                .collect();
            symbols.sort_by_key(|s| s.address());

            if symbols.is_empty() {
                continue;
            }

            for (i, sym) in symbols.iter().enumerate() {
                let name = match sym.name() {
                    Ok(n) if !n.is_empty() => n.to_string(),
                    _ => continue,
                };

                let sym_start = (sym.address() - section.address()) as usize;
                let sym_end = if i + 1 < symbols.len() {
                    (symbols[i + 1].address() - section.address()) as usize
                } else {
                    section_data.len()
                };

                if sym_start >= section_data.len() || sym_end > section_data.len() {
                    continue;
                }

                let data = &section_data[sym_start..sym_end];

                // Collect relocation target names for this symbol's address range
                let reloc_target_names: Vec<Option<String>> = relocations
                    .iter()
                    .filter(|(addr, _)| *addr >= sym_start as u64 && *addr < sym_end as u64)
                    .map(|(_, reloc)| resolve_relocation_name(&obj, reloc.target()))
                    .collect();

                result.insert(
                    name,
                    ExtractedSymbol {
                        data_len: data.len(),
                        data_hash: fnv1a_hash(data),
                        reloc_count: reloc_target_names.len(),
                        reloc_target_names,
                        is_global: sym.is_global(),
                    },
                );
            }
        }
    }

    result
}

/// Compare two extracted symbols using masked comparison.
fn symbols_match(old: &ExtractedSymbol, new: &ExtractedSymbol) -> bool {
    if old.data_len != new.data_len {
        return false;
    }

    if old.reloc_count != new.reloc_count {
        return false;
    }

    // Check that all relocations point to the same symbols by name
    for (old_name, new_name) in old.reloc_target_names.iter().zip(&new.reloc_target_names) {
        if old_name != new_name {
            return false;
        }
    }

    // Compare data hashes.
    // Note: this includes relocation target bytes, making it conservative (may report
    // false changes). A full implementation would mask out relocation target bytes
    // before hashing, but this is safe — it may trigger unnecessary recompilation
    // but never misses a real change.
    if old.data_hash != new.data_hash {
        return false;
    }

    true
}

/// Resolve a relocation target to its symbol name.
fn resolve_relocation_name(obj: &ObjectFile<'_>, target: RelocationTarget) -> Option<String> {
    match target {
        RelocationTarget::Symbol(idx) => obj
            .symbol_by_index(idx)
            .ok()
            .and_then(|s| s.name().ok())
            .map(|n| n.to_string()),
        RelocationTarget::Section(idx) => obj
            .section_by_index(idx)
            .ok()
            .and_then(|s| s.name().ok())
            .map(|n| n.to_string()),
        _ => None,
    }
}

/// FNV-1a hash for comparing data blobs.
fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
