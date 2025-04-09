use anyhow::{Context, Result};
use itertools::Itertools;
use memmap::{Mmap, MmapOptions};
use object::{
    macho::{self, ARM64_RELOC_UNSIGNED, MH_TWOLEVEL},
    read::File,
    write::{MachOBuildVersion, Relocation, StandardSection, Symbol, SymbolSection},
    Architecture, BinaryFormat, Endianness, Object, ObjectSection, ObjectSymbol, ObjectSymbolTable,
    RelocationFlags, RelocationTarget, SectionIndex, SectionKind, SymbolFlags, SymbolKind,
    SymbolScope,
};
use std::{cmp::Ordering, ffi::OsStr, fs, ops::Deref, path::PathBuf};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
};
use std::{io::Write, os::raw::c_void};
pub use subsecond_types::*;
pub use subsecond_types::*;
use target_lexicon::{OperatingSystem, Triple};
use tokio::process::Command;
use walkdir::WalkDir;
use wasm_encoder::{CustomSection, DataSymbolDefinition, Encode, LinkingSection, SymbolTable};
use wasmparser::{
    BinaryReader, Linking, LinkingSectionReader, Payload, RelocSectionReader, RelocationEntry,
    SymbolInfo,
};

use walrus::{
    ir::{dfs_in_order, Visitor},
    ElementItems, ElementKind, FunctionId, FunctionKind, IdsToIndices, ImportKind, Module,
    ModuleConfig, RawCustomSection, ValType,
};

pub mod partial;

pub fn create_jump_table(
    original: &Path,
    patch: &Path,
    triple: &Triple,
) -> anyhow::Result<JumpTable> {
    if triple.architecture == target_lexicon::Architecture::Wasm32 {
        return create_wasm_jump_table(original, patch);
    }

    let obj1_bytes = fs::read(original).context("Could not read original file")?;
    let obj2_bytes = fs::read(patch).context("Could not read patch file")?;
    let obj1 = File::parse(&obj1_bytes as &[u8]).unwrap();
    let obj2 = File::parse(&obj2_bytes as &[u8]).unwrap();

    let mut map = AddressMap::default();

    let old_syms = obj1.symbol_map();
    let new_syms = obj2.symbol_map();

    let old_name_to_addr = old_syms
        .symbols()
        .iter()
        .map(|s| (s.name(), s.address()))
        .collect::<HashMap<_, _>>();

    let new_name_to_addr = new_syms
        .symbols()
        .iter()
        .map(|s| (s.name(), s.address()))
        .collect::<HashMap<_, _>>();

    for (new_name, new_addr) in new_name_to_addr.iter() {
        if let Some(old_addr) = old_name_to_addr.get(new_name) {
            map.insert(*old_addr, *new_addr);
        }
    }

    // on windows there is no symbol so we leave the old address as 0
    // on wasm there is no ASLR so we leave the old address as 0
    let mut old_base_address = 0;
    let mut new_base_address = 0;
    match triple.operating_system {
        OperatingSystem::Darwin(_)
        | OperatingSystem::Linux
        | OperatingSystem::MacOSX(_)
        | OperatingSystem::IOS(_)
        | OperatingSystem::Windows => {
            let options = ["___rust_alloc", "__rust_alloc"];
            for option in options {
                if old_name_to_addr.contains_key(option) {
                    old_base_address = old_name_to_addr.get(option).unwrap().clone();
                    new_base_address = new_name_to_addr.get(option).unwrap().clone();
                    break;
                }
            }
        }
        _ => {}
    }

    let aslr_reference = old_name_to_addr
        .get("aslr_reference")
        .unwrap_or_else(|| {
            old_name_to_addr
                .get("_aslr_reference")
                .expect("failed to find aslr_reference")
        })
        .clone();

    Ok(JumpTable {
        lib: patch.to_path_buf(),
        map,
        new_base_address,
        aslr_reference,
        ifunc_count: 0,
    })
}

/// In the web, our patchable functions are actually ifuncs
///
/// We need to line up the ifuncs from the main module to the ifuncs in the patch.
fn create_wasm_jump_table(original: &Path, patch: &Path) -> anyhow::Result<JumpTable> {
    tracing::info!("jumping {} to {}", original.display(), patch.display());
    let obj1_bytes = fs::read(original).context("Could not read original file")?;
    let obj2_bytes = fs::read(patch).context("Could not read patch file")?;

    let old = walrus::Module::from_buffer(&obj1_bytes)?;
    let new = walrus::Module::from_buffer(&obj2_bytes)?;

    let name_to_ifunc_old = collect_func_ifuncs(&old);
    let name_to_ifunc_new = collect_func_ifuncs(&new);

    let mut map = AddressMap::default();
    for (name, idx) in name_to_ifunc_new.iter() {
        if let Some(old_idx) = name_to_ifunc_old.get(name) {
            map.insert(*old_idx as u64, *idx as u64);
        }
    }

    Ok(JumpTable {
        map,
        lib: patch.to_path_buf(),
        aslr_reference: 0,
        new_base_address: 0,
        ifunc_count: name_to_ifunc_new.len() as u64,
    })
}

fn collect_func_ifuncs(m: &Module) -> HashMap<&str, i32> {
    let mut name_to_ifunc_index = HashMap::new();

    for el in m.elements.iter() {
        let ElementKind::Active { offset, .. } = &el.kind else {
            continue;
        };

        let offset = match offset {
            // Handle explicit offsets
            walrus::ConstExpr::Value(value) => match value {
                walrus::ir::Value::I32(idx) => *idx,
                walrus::ir::Value::I64(idx) => *idx as i32,
                _ => continue,
            },

            // Globals are usually imports and thus don't add a specific offset
            // ie the ifunc table is offset by a global, so we don't need to push the offset out
            walrus::ConstExpr::Global(_) => 0,

            walrus::ConstExpr::RefNull(_) => continue,
            walrus::ConstExpr::RefFunc(_) => continue,
        };

        if let ElementItems::Functions(ids) = &el.items {
            for (idx, id) in ids.iter().enumerate() {
                let name = m.funcs.get(*id).name.as_ref().unwrap();
                name_to_ifunc_index.insert(name.as_str(), offset + idx as i32);
            }
        }
    }

    name_to_ifunc_index
}

/// Resolve the undefined symbols in the incrementals against the original binary, returning an object
/// file that can be linked along the incrementals.
///
/// This makes it possible to dlopen the resulting object file and use the original binary's symbols
/// bypassing the dynamic linker.
///
/// This is very similar to malware :) but it's not!
pub fn resolve_undefined(
    source_path: &Path,
    incrementals: &[PathBuf],
    triple: &Triple,
    aslr_reference: u64,
) -> Result<Vec<u8>> {
    let sorted: Vec<_> = incrementals.iter().sorted().collect();

    // Find all the undefined symbols in the incrementals
    let mut undefined_symbols = HashSet::new();
    let mut defined_symbols = HashSet::new();
    for path in sorted {
        let bytes = fs::read(&path).with_context(|| format!("failed to read {:?}", path))?;
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
            target_lexicon::BinaryFormat::Coff => object::BinaryFormat::Coff,
            target_lexicon::BinaryFormat::Wasm => object::BinaryFormat::Wasm,
            target_lexicon::BinaryFormat::Xcoff => object::BinaryFormat::Xcoff,
            _ => todo!(),
        },
        match triple.architecture {
            target_lexicon::Architecture::Aarch64(_) => object::Architecture::Aarch64,
            target_lexicon::Architecture::Wasm32 => object::Architecture::Wasm32,
            target_lexicon::Architecture::X86_64 => object::Architecture::X86_64,
            _ => todo!(),
        },
        match triple.endianness() {
            Ok(target_lexicon::Endianness::Little) => Endianness::Little,
            Ok(target_lexicon::Endianness::Big) => Endianness::Big,
            _ => Endianness::Little,
        },
    );

    // Write the headers so we load properly in ios/macos
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
                build_version.platform = match triple.environment {
                    target_lexicon::Environment::Sim => macho::PLATFORM_IOSSIMULATOR,
                    _ => macho::PLATFORM_IOS,
                };
                build_version.minos = (14 << 16) | (0 << 8) | 0; // 14.0.0
                build_version.sdk = (14 << 16) | (0 << 8) | 0; // SDK 14.0.0
                build_version
            });
        }

        _ => {}
    }

    // Load the original binary
    let bytes =
        fs::read(&source_path).with_context(|| format!("failed to read {:?}", source_path))?;
    let source = File::parse(bytes.deref() as &[u8])?;
    let symbol_table = source
        .symbols()
        .flat_map(|s| Some((s.name().ok()?, s)))
        .collect::<HashMap<_, _>>();

    // Get the offset from the main module
    let aslr_offset = match triple.architecture {
        target_lexicon::Architecture::Wasm32 => 0,
        _ => {
            let aslr_ref = symbol_table.get("_aslr_reference").unwrap_or_else(|| {
                symbol_table
                    .get("aslr_reference")
                    .expect("failed to find aslr_reference")
            });
            aslr_reference - aslr_ref.address()
        }
    };

    // we need to assemble a PLT/GOT so direct calls to the patch symbols work
    // for each symbol we either write the address directly (as a symbol) or create a PLT/GOT entry
    let text_section = obj.section_id(StandardSection::Text);
    for name in undefined_symbols {
        let Some(sym) = symbol_table.get(name.as_str()) else {
            tracing::error!("Symbol not found: {}", name);
            continue;
        };

        if sym.is_undefined() {
            tracing::debug!("Skipping undefined symbol {name}");
            continue;
        }

        let name_offset = match triple.operating_system {
            target_lexicon::OperatingSystem::Darwin(_) => 1,
            target_lexicon::OperatingSystem::IOS(_) => 1,
            _ => 0,
        };

        let abs_addr = sym.address() + aslr_offset;

        tracing::debug!("Defining: {:?}", name);

        if sym.kind() == SymbolKind::Text {
            let jump_code = match triple.architecture {
                target_lexicon::Architecture::X86_64 => {
                    // Use JMP instruction to absolute address: FF 25 followed by 32-bit offset
                    // Then the 64-bit absolute address
                    let mut code = vec![0xFF, 0x25, 0x00, 0x00, 0x00, 0x00]; // jmp [rip+0]
                                                                             // Append the 64-bit address
                    code.extend_from_slice(&abs_addr.to_le_bytes());
                    code
                }
                target_lexicon::Architecture::X86_32(_) => {
                    // For 32-bit Intel, use JMP instruction with absolute address
                    let mut code = vec![0xE9]; // jmp rel32
                    let rel_addr = (abs_addr as i32 - 5) as i32; // Relative address (offset from next instruction)
                    code.extend_from_slice(&rel_addr.to_le_bytes());
                    code
                }
                target_lexicon::Architecture::Aarch64(_) => {
                    // For ARM64, we load the address into a register and branch
                    let mut code = Vec::new();
                    // LDR X16, [PC, #0]  ; Load from the next instruction
                    code.extend_from_slice(&[0x50, 0x00, 0x00, 0x58]);
                    // BR X16            ; Branch to the address in X16
                    code.extend_from_slice(&[0x00, 0x02, 0x1F, 0xD6]);
                    // Store the 64-bit address
                    code.extend_from_slice(&abs_addr.to_le_bytes());
                    code
                }
                target_lexicon::Architecture::Arm(_) => {
                    // For 32-bit ARM, use LDR PC, [PC, #-4] to load the address and branch
                    let mut code = Vec::new();
                    // LDR PC, [PC, #-4] ; Load the address into PC (branching to it)
                    code.extend_from_slice(&[0x04, 0xF0, 0x1F, 0xE5]);
                    // Store the 32-bit address
                    code.extend_from_slice(&(abs_addr as u32).to_le_bytes());
                    code
                }
                // Add other architectures as needed
                _ => todo!(),
            };

            // Add the jump code to the text section
            let offset = obj.append_section_data(text_section, &jump_code, 8);
            obj.add_symbol(Symbol {
                name: name.as_bytes()[name_offset..].to_vec(),
                value: offset,
                size: jump_code.len() as u64,
                scope: SymbolScope::Linkage,
                kind: SymbolKind::Text,
                weak: false,
                section: SymbolSection::Section(text_section),
                flags: object::SymbolFlags::None,
            });
        } else {
            obj.add_symbol(Symbol {
                name: name.as_bytes()[name_offset..].to_vec(),
                value: abs_addr,
                size: 0,
                scope: SymbolScope::Linkage,
                kind: sym.kind(),
                weak: sym.is_weak(),
                section: SymbolSection::Absolute,
                flags: object::SymbolFlags::None,
            });
        }
    }

    // Write the object to a file
    let bytes = obj.write()?;
    Ok(bytes)
}

/// Prepares the base module before running wasm-bindgen.
///
/// This tries to work around how wasm-bindgen works by intelligently promoting non-wasm-bindgen functions
/// to the export table.
pub fn prepare_wasm_base_module(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut pre_bindgen = walrus::Module::from_buffer(bytes)?;

    let bindgen_funcs = collect_all_wasm_bindgen_funcs(&pre_bindgen);

    // Due to monomorphizations, functions will get merged and multiple names will point to the same function.
    // Walrus loses this information, so we need to manually parse the names table to get the indices
    // and names of these functions.
    let raw_data = parse_bytes_to_data_segment(bytes)?;

    // name -> index
    // we want to export *all* these functions
    let all_funcs = raw_data
        .iter()
        .flat_map(|sym| match sym {
            SymbolInfo::Func { index, name, .. } => Some((name.unwrap(), *index)),
            _ => None,
        })
        .collect::<HashMap<_, _>>();

    let index_to_func = pre_bindgen
        .funcs
        .iter()
        .enumerate()
        .collect::<HashMap<_, _>>();

    let mut already_exported = pre_bindgen
        .exports
        .iter()
        .map(|exp| exp.name.clone())
        .chain(
            bindgen_funcs
                .iter()
                .map(|id| pre_bindgen.funcs.get(*id).name.as_ref().unwrap().clone()),
        )
        .collect::<HashSet<_>>();

    for (name, index) in all_funcs {
        let func = index_to_func.get(&(index as usize)).unwrap();
        if let FunctionKind::Local(_local) = &func.kind {
            if !already_exported.contains(name) {
                pre_bindgen.exports.add(&name, func.id());
                already_exported.insert(name.to_string());
            }
        }
    }

    Ok(pre_bindgen.emit_wasm())
}

/// Collect all the wasm-bindgen functions in the module. We are going to make *everything* exported
/// but we don't want to make *these* exported.
fn collect_all_wasm_bindgen_funcs(module: &Module) -> HashSet<FunctionId> {
    /// The __wbindgen_describe_ functions also reference funcs like:
    /// _ZN86_$LT$dioxus_web..document..JSOwner$u20$as$u20$wasm_bindgen..describe..WasmDescribe$GT$8describe17ha9b39368d518c1f9E
    ///
    /// These can be found by walking the instructions, so we build a Visitor
    /// ... todo: we might not need to do this since it seems that it's not reliable enough
    #[derive(Default)]
    struct AccAllDescribes {
        funcs: HashSet<FunctionId>,
    }

    impl<'a> Visitor<'a> for AccAllDescribes {
        fn visit_function_id(&mut self, function: &walrus::FunctionId) {
            self.funcs.insert(*function);
        }
    }

    let mut acc = AccAllDescribes::default();
    for func in module.funcs.iter() {
        let name = func.name.as_ref().unwrap();

        // Only deal with the __wbindgen_describe_ functions
        if !(name.starts_with("__wbindgen_describe_")
            || name.contains("wasm_bindgen..describe..WasmDescribe")
            || name.contains("wasm_bindgen..closure..WasmClosure$GT$8describe")
            || name.contains("wasm_bindgen7closure16Closure$LT$T$GT$4wrap8describe")
            || name.contains("__wbindgen_describe_closure")
            || name.contains("__wbindgen_externref_xform"))
        {
            continue;
        }

        // They call other functions, so we need to find those too and make sure not to mark them as exported
        if let FunctionKind::Local(func) = &module.funcs.get(func.id()).kind {
            dfs_in_order(&mut acc, &func, func.entry_block());
        }

        acc.funcs.insert(func.id());
    }

    acc.funcs
}

/// According to the dylink spec, there will be two sets of entries:
/// - got.func: functions in the indirect function table
/// - got.mem: data objects in the data segments
///
/// It doesn't seem like we can compile the base module to export these, sadly, so we're going
/// to manually satisfy them here, removing their need to be imported.
///
/// https://github.com/WebAssembly/tool-conventions/blob/main/DynamicLinking.md
pub fn satisfy_got_imports(old_bytes: &[u8], new_bytes: &[u8]) -> Result<Vec<u8>> {
    let old: walrus::Module = walrus::Module::from_buffer(old_bytes)?;
    let mut new: walrus::Module = walrus::Module::from_buffer(new_bytes)?;

    let ifunc_map = collect_func_ifuncs(&old);
    let global_map = collect_global_map(&old);

    let mut mems = vec![];
    let mut funcs = vec![];

    // Collect the GOT func/mem entries
    for t in new.imports.iter() {
        match t.module.as_str() {
            "GOT.func" => funcs.push((t.id(), *ifunc_map.get(t.name.as_str()).unwrap())),
            "GOT.mem" => mems.push(t.id()),
            _ => {}
        }
    }

    // Satisfies the GOT.func imports
    for (imp_id, val) in funcs {
        let imp = new.imports.get(imp_id);
        let global_id = match imp.kind {
            ImportKind::Global(id) => id,
            _ => todo!(),
        };
        new.globals.get_mut(global_id).kind =
            walrus::GlobalKind::Local(walrus::ConstExpr::Value(walrus::ir::Value::I32(val as i32)));
        new.imports.delete(imp_id);
    }

    // The got mem entries exist, but are hidden. we need to bind to their address directly, and
    // remove the "GOT.data.internal" name
    for mem in mems {
        let imp = new.imports.get(mem);
        let name = format!("GOT.data.internal.{}", imp.name);
        let val = global_map.get(name.as_str()).unwrap();
        let global_id = match imp.kind {
            ImportKind::Global(id) => id,
            _ => todo!(),
        };
        new.globals.get_mut(global_id).kind =
            walrus::GlobalKind::Local(walrus::ConstExpr::Value(walrus::ir::Value::I32(*val)));
        new.imports.delete(mem);
    }

    Ok(new.emit_wasm())
}

fn collect_global_map(old: &Module) -> HashMap<&str, i32> {
    let mut global_map = HashMap::new();

    for global in old.globals.iter() {
        if let Some(name) = &global.name {
            if let walrus::GlobalKind::Local(walrus::ConstExpr::Value(walrus::ir::Value::I32(
                value,
            ))) = global.kind
            {
                global_map.insert(name.as_str(), value);
            }
        }
    }

    global_map
}

/// Manually parse the data section from a wasm module
///
/// We need to do this for data symbols because walrus doesn't provide the right range and offset
/// information for data segments. Fortunately, it provides it for code sections, so we only need to
/// do a small amount extra of parsing here.
fn parse_bytes_to_data_segment(bytes: &[u8]) -> Result<Vec<SymbolInfo>> {
    let parser = wasmparser::Parser::new(0);
    let mut parser = parser.parse_all(bytes);
    let mut symbols = vec![];

    // Process the payloads in the raw wasm file so we can extract the specific sections we need
    while let Some(Ok(payload)) = parser.next() {
        match payload {
            Payload::CustomSection(section) if section.name() == "linking" => {
                let reader = BinaryReader::new(section.data(), 0);
                let reader = LinkingSectionReader::new(reader)?;
                for subsection in reader.subsections() {
                    if let Linking::SymbolTable(map) = subsection? {
                        symbols = map.into_iter().collect::<Result<Vec<_>, _>>()?;
                    }
                }
            }
            _ => {}
        }
    }

    Ok(symbols)
}
