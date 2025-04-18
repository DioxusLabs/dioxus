use anyhow::{Context, Result};
use itertools::Itertools;
use memmap::{Mmap, MmapOptions};
use object::{
    macho::{self, ARM64_RELOC_UNSIGNED, MH_TWOLEVEL},
    read::{File, Relocation as ReadRelocation},
    write::{
        MachOBuildVersion, Relocation as WriteRelocation, SectionId, StandardSection, Symbol,
        SymbolSection,
    },
    Architecture, BinaryFormat, Endianness, Object, ObjectSection, ObjectSymbol, ObjectSymbolTable,
    RelocationFlags, RelocationTarget, SectionIndex, SectionKind, SymbolFlags, SymbolKind,
    SymbolScope,
};
use std::{cmp::Ordering, ffi::OsStr, fs, ops::Deref, panic, path::PathBuf};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
};
use std::{io::Write, os::raw::c_void};
use subsecond_types::*;
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

pub fn create_jump_table(
    original: &Path,
    patch: &Path,
    triple: &Triple,
) -> anyhow::Result<JumpTable> {
    // WASM needs its own path since the object crate leaves quite a few of the methods unimplemented
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
        .get("_aslr_reference")
        .unwrap_or_else(|| {
            old_name_to_addr
                .get("aslr_reference")
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

    let old_raw_data = parse_bytes_to_data_segment(&obj1_bytes)?;
    let new_raw_data = parse_bytes_to_data_segment(&obj2_bytes)?;

    let name_to_ifunc_old = collect_func_ifuncs(&old, &old_raw_data);
    let name_to_ifunc_new = collect_func_ifuncs(&new, &new_raw_data);

    let mut map = AddressMap::default();
    for (name, idx) in name_to_ifunc_new.iter() {
        if let Some(old_idx) = name_to_ifunc_old.get(name) {
            map.insert(*old_idx as u64, *idx as u64);
        }
    }

    tracing::trace!("Jump table: {:#?}", map);

    Ok(JumpTable {
        map,
        lib: patch.to_path_buf(),
        aslr_reference: 0,
        new_base_address: 0,
        ifunc_count: name_to_ifunc_new.len() as u64,
    })
}

fn collect_func_ifuncs<'a>(m: &'a Module, syms: &'a [SymbolInfo<'a>]) -> HashMap<&'a str, i32> {
    // name -> index
    // we want to export *all* these functions
    let namemap = fn_name_map(syms);
    let mut indexes_to_names = HashMap::<WrongFnIndex, Vec<&str>>::new();
    for (n, i) in namemap.iter() {
        indexes_to_names.entry(*i).or_default().push(*n);
    }

    let mut offsets = HashMap::new();

    for el in m.elements.iter() {
        let ElementKind::Active { offset, .. } = &el.kind else {
            tracing::info!("Skipping section: {:?} -> {:?}", el.name, el.kind);
            continue;
        };

        let offset = match offset {
            // Handle explicit offsets
            walrus::ConstExpr::Value(value) => match value {
                walrus::ir::Value::I32(idx) => *idx,
                walrus::ir::Value::I64(idx) => *idx as i32,
                _ => panic!(),
            },

            // Globals are usually imports and thus don't add a specific offset
            // ie the ifunc table is offset by a global, so we don't need to push the offset out
            walrus::ConstExpr::Global(_) => 0,
            walrus::ConstExpr::RefNull(_) => panic!(),
            walrus::ConstExpr::RefFunc(_) => panic!(),
        };

        match &el.items {
            ElementItems::Functions(ids) => {
                for (idx, id) in ids.iter().enumerate() {
                    let f = m.funcs.get(*id);
                    offsets.insert(f.name.as_deref().unwrap(), offset + idx as i32);
                }
            }
            ElementItems::Expressions(ref_type, const_exprs) => {}
        }
    }

    offsets
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct WrongFnIndex(u32);
fn fn_name_map<'a>(syms: &[SymbolInfo<'a>]) -> HashMap<&'a str, WrongFnIndex> {
    let all_funcs = syms
        .iter()
        .flat_map(|sym| match sym {
            SymbolInfo::Func { index, name, .. } => Some((name.unwrap(), WrongFnIndex(*index))),
            _ => None,
        })
        .collect::<HashMap<_, _>>();
    all_funcs
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

        tracing::trace!("Defining: {:?} -> {:?}", name, sym.kind());

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
            // It's likely a static
            if sym.kind() == SymbolKind::Unknown {
                obj.add_symbol(Symbol {
                    name: name.as_bytes()[name_offset..].to_vec(),
                    value: abs_addr,
                    size: 0,
                    scope: SymbolScope::Linkage,
                    kind: SymbolKind::Data,
                    weak: false,
                    section: SymbolSection::Absolute,
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
    }

    Ok(obj.write()?)
}

/// Prepares the base module before running wasm-bindgen.
///
/// This tries to work around how wasm-bindgen works by intelligently promoting non-wasm-bindgen functions
/// to the export table.
///
/// It also moves all functions and memories to be callable indirectly.
pub fn prepare_wasm_base_module(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut module = walrus::Module::from_buffer(bytes)?;

    let bindgen_funcs = collect_all_wasm_bindgen_funcs(&module);

    // Due to monomorphizations, functions will get merged and multiple names will point to the same function.
    // Walrus loses this information, so we need to manually parse the names table to get the indices
    // and names of these functions.
    //
    // Unfortunately, the indicies it gives us ARE NOT VALID.
    // We need to work around it by using the FunctionId from the module as a link between the merged function names.
    let raw_data = parse_bytes_to_data_segment(bytes)?;
    let ifunc_map = collect_func_ifuncs(&module, &raw_data);
    let ifuncs = module
        .funcs
        .iter()
        .filter_map(|f| ifunc_map.get(f.name.as_deref()?).map(|_| f.id()))
        .collect::<HashSet<_>>();
    let ifunc_table_initialzer = module.elements.iter().last().unwrap().id();

    let all_funcs = fn_name_map(&raw_data);
    let wrong_to_right = module
        .funcs
        .iter()
        .filter_map(|f| {
            let name = f.name.as_deref().unwrap();
            Some((all_funcs.get(name)?.clone(), f.id()))
        })
        .collect::<HashMap<_, _>>();

    let mut already_exported = module
        .exports
        .iter()
        .map(|exp| exp.name.clone())
        .chain(
            bindgen_funcs
                .iter()
                .map(|id| module.funcs.get(*id).name.as_ref().unwrap().clone()),
        )
        .collect::<HashSet<_>>();

    let mut make_indirect = vec![];

    for (name, wrong_idx) in all_funcs.iter() {
        let f = wrong_to_right.get(wrong_idx).unwrap().clone();
        if bindgen_funcs.contains(&f) {
            continue;
        }
        let func = module.funcs.get(f);
        if let FunctionKind::Local(_local) = &func.kind {
            if !already_exported.contains(*name) {
                module.exports.add(*name, func.id());
                already_exported.insert(name.to_string());
            }

            if !ifuncs.contains(&f) {
                make_indirect.push(func.id());
            }
        }
    }

    tracing::trace!("Hoisting {} functions", make_indirect.len());
    let seg = module.elements.get_mut(ifunc_table_initialzer);
    let make_indirect_count = make_indirect.len() as u64;
    match &mut seg.items {
        ElementItems::Functions(ids) => {
            for func in make_indirect {
                ids.push(func);
            }
        }
        ElementItems::Expressions(ref_type, const_exprs) => {
            panic!("Indirect function table is not a function table: {const_exprs:?}")
        }
    };

    let table = match seg.kind {
        ElementKind::Active { table, offset } => table,
        _ => todo!(),
    };

    let table = module.tables.get_mut(table);
    table.initial += make_indirect_count;
    if let Some(max) = table.maximum {
        table.maximum = Some(max + make_indirect_count);
    }

    Ok(module.emit_wasm())
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

    let raw_data = parse_bytes_to_data_segment(&old_bytes)?;
    let ifunc_map = collect_func_ifuncs(&old, &raw_data);
    let global_map = collect_global_map(&old);

    let mut mems = vec![];
    let mut funcs = vec![];

    // Collect the GOT func/mem entries
    for t in new.imports.iter() {
        match t.module.as_str() {
            "GOT.func" => {
                funcs.push((
                    t.id(),
                    *ifunc_map.get(t.name.as_str()).unwrap_or_else(|| {
                        let exists = old.exports.get_func(t.name.as_str());
                        panic!("failed to find GOT.func: {} -> {exists:?}", t.name.as_str())
                    }),
                ));
            }
            "GOT.mem" => mems.push(t.id()),
            _ => {}
        }
    }

    // Satisfies the GOT.func imports. They exist as regular imports, but we need to make the indirect call
    for (imp_id, val) in funcs {
        // for (import_id, name) in funcs {
        let imp = new.imports.get(imp_id);
        let global_id = match imp.kind {
            ImportKind::Global(id) => id,
            _ => todo!(),
        };
        // new.globals.get_mut(global_id).kind =
        //     walrus::GlobalKind::Local(walrus::ConstExpr::Value(walrus::ir::Value::I32(val as i32)));
        //     new.imports.delete(imp_id);
        new.globals.get_mut(global_id).kind =
            walrus::GlobalKind::Local(walrus::ConstExpr::Value(walrus::ir::Value::I32(val as i32)));
        new.imports.delete(imp_id);
    }

    // The got mem entries exist, but are hidden. we need to bind to their address directly, and
    // remove the "GOT.data.internal" name
    for mem in mems {
        let imp = new.imports.get(mem);
        let name = format!("GOT.data.internal.{}", imp.name);
        let val = global_map.get(name.as_str()).unwrap_or_else(|| {
            let non_got = global_map.get(name.as_str());
            panic!(
                "failed to find GOT.mem: {} -> non got: {non_got:?}",
                name.as_str()
            )
        });
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

struct SymbolMap<'a> {
    symbols: Vec<SymbolInfo<'a>>,
}

enum Node {
    Function(FunctionId),
    Data(usize),
}

async fn attempt_partial_link(proc_main_addr: u64, patch_target: PathBuf, out_path: PathBuf) {
    let mut object = ObjectDiff::new().unwrap();
    object.load().unwrap();
    let diff = object.diff().unwrap();

    // Assemble the stub
    let stub_data = make_stub_file(proc_main_addr, patch_target, diff.adrp_imports);
    let stub_file = workspace_dir().join("stub.o");
    std::fs::write(&stub_file, stub_data).unwrap();
}

struct ObjectDiffResult<'a> {
    adrp_imports: HashSet<&'a str>,
    modified_files: Vec<(&'a PathBuf, &'a HashSet<String>)>,
    modified_symbols: HashSet<&'a String>,
}

struct ObjectDiff {
    old: BTreeMap<String, LoadedFile>,
    new: BTreeMap<String, LoadedFile>,
    modified_files: HashMap<PathBuf, HashSet<String>>,
    modified_symbols: HashSet<String>,
    parents: HashMap<String, HashSet<String>>,
}

impl ObjectDiff {
    fn new() -> Result<Self> {
        Ok(Self {
            old: LoadedFile::from_dir(&workspace_dir().join("data").join("incremental-old"))?,
            new: LoadedFile::from_dir(&workspace_dir().join("data").join("incremental-new"))?,
            modified_files: Default::default(),
            modified_symbols: Default::default(),
            parents: Default::default(),
        })
    }

    fn diff(&self) -> Result<ObjectDiffResult<'_>> {
        let all_exports = self
            .new
            .iter()
            .flat_map(|(_, f)| f.file.exports().unwrap())
            .map(|e| e.name().to_utf8())
            .collect::<HashSet<_>>();

        let mut adrp_imports = HashSet::new();

        let mut satisfied_exports = HashSet::new();

        let modified_symbols = self.modified_symbols.iter().collect::<HashSet<_>>();

        if modified_symbols.is_empty() {
            println!("No modified symbols");
        }

        let mut modified_log = String::new();
        for m in modified_symbols.iter() {
            let path = self.find_path_to_main(m);
            modified_log.push_str(&format!("{m}\n"));
            modified_log.push_str(&format!("{path:#?}\n"));
        }
        std::fs::write(workspace_dir().join("modified_symbols.txt"), modified_log).unwrap();

        let modified = self
            .modified_files
            .iter()
            .sorted_by(|a, b| a.0.cmp(&b.0))
            .collect::<Vec<_>>();

        // Figure out which symbols are required from *existing* code
        // We're going to create a stub `.o` file that satisfies these by jumping into the original code via a dynamic lookup / and or literally just manually doing it
        for fil in modified.iter() {
            let f = self
                .new
                .get(fil.0.file_name().unwrap().to_str().unwrap())
                .unwrap();

            for i in f.file.imports().unwrap() {
                if all_exports.contains(i.name().to_utf8()) {
                    adrp_imports.insert(i.name().to_utf8());
                }
            }

            for e in f.file.exports().unwrap() {
                satisfied_exports.insert(e.name().to_utf8());
            }
        }

        // Remove any imports that are indeed satisifed
        for s in satisfied_exports.iter() {
            adrp_imports.remove(s);
        }

        Ok(ObjectDiffResult {
            adrp_imports,
            modified_files: modified,
            modified_symbols,
        })
    }

    fn load(&mut self) -> Result<()> {
        let num_right = self.new.len();

        let keys = self.new.keys().cloned().collect::<Vec<_>>();
        for (idx, f) in keys.iter().enumerate() {
            println!("----- {:?} {}/{} -----", f, idx, num_right);

            let changed_before = self.modified_symbols.len();
            self.load_file(f)?;
            let changed_after = self.modified_symbols.len();

            if changed_after > changed_before {
                println!("❌ -> {}", changed_after - changed_before);
            }
        }

        Ok(())
    }

    /// Walk the call  to find the path to the main function
    fn find_path_to_main(&self, name: &str) -> Vec<String> {
        let mut path = Vec::new();
        let mut visited = std::collections::HashSet::new();

        // Helper function for DFS with backtracking
        fn dfs(
            current: &str,
            path: &mut Vec<String>,
            visited: &mut std::collections::HashSet<String>,
            parents: &std::collections::HashMap<String, HashSet<String>>,
        ) -> bool {
            // If we've found main, we're done
            if current.ends_with("_main") {
                path.push(current.to_string());
                return true;
            }

            // Mark current node as visited
            visited.insert(current.to_string());
            path.push(current.to_string());

            // Check all parents of the current node
            if let Some(parent_nodes) = parents.get(current) {
                for parent in parent_nodes {
                    if !visited.contains(parent) {
                        if dfs(parent, path, visited, parents) {
                            return true;
                        }
                    }
                }
            }

            // If no path is found through this node, backtrack
            path.pop();

            false
        }

        // Start DFS from the given name
        dfs(name, &mut path, &mut visited, &self.parents);

        path
    }

    fn load_file(&mut self, name: &str) -> Result<()> {
        let new = &self.new[name];
        let Some(old) = self.old.get(name) else {
            self.modified_files.entry(new.path.clone()).or_default();
            return Ok(());
        };

        let mut changed_list = HashSet::new();
        for section in new.file.sections() {
            let n = section.name().unwrap();
            if n == "__text"
                || n == "__const"
                || n.starts_with("__literal")
                || n == "__eh_frame"
                || n == "__compact_unwind"
                || n == "__gcc_except_tab"
                || n == "__common"
                || n == "__bss"
            {
                changed_list.extend(self.accumulate_changed(&old, &new, section.index()));
            } else {
                println!("Skipping section: {n}");
            }
        }

        for c in changed_list.iter() {
            if !c.starts_with("l") && !c.starts_with("ltmp") {
                self.modified_symbols.insert(c.to_string());
            } else {
                let mod_name = format!("{c}_{name}");
                self.modified_symbols.insert(mod_name);
            }
        }

        for (child, parents) in new.parents.iter() {
            let child_name = match child.starts_with("l") {
                true => format!("{child}_{name}"),
                false => child.to_string(),
            };

            for parent in parents {
                let p_name = match parent.starts_with("l") {
                    true => format!("{parent}_{name}"),
                    false => parent.to_string(),
                };

                self.parents
                    .entry(child_name.clone())
                    .or_default()
                    .insert(p_name);
            }
        }

        Ok(())
    }

    fn accumulate_changed(
        &self,
        old: &LoadedFile,
        new: &LoadedFile,
        section_idx: SectionIndex,
    ) -> HashSet<&'static str> {
        let mut local_modified = HashSet::new();

        // Accumulate modified symbols using masking in functions
        let relocated_new = acc_symbols(&new.file, section_idx);
        let mut relocated_old = acc_symbols(&old.file, section_idx)
            .into_iter()
            .map(|f| (f.name, f))
            .collect::<HashMap<_, _>>();

        for right in relocated_new {
            let Some(left) = relocated_old.remove(right.name) else {
                local_modified.insert(right.name);
                continue;
            };

            // If the contents of the assembly changed, track it
            if !compare_masked(old.file, new.file, &left, &right) {
                local_modified.insert(left.name);
                local_modified.insert(right.name);
            }
        }

        local_modified
    }
}

/// A file loaded into memory with its analysis
///
/// We leak the module to make it easier to deal with its contents
struct LoadedFile {
    path: PathBuf,
    open_file: std::fs::File,
    mmap: &'static Mmap,

    file: &'static File<'static>,

    // symbol -> symbols
    parents: HashMap<&'static str, HashSet<&'static str>>,
}

impl LoadedFile {
    fn from_dir(dir: &Path) -> anyhow::Result<BTreeMap<String, Self>> {
        std::fs::read_dir(dir)?
            .into_iter()
            .flatten()
            .filter(|e| e.path().extension() == Some(OsStr::new("o")))
            .map(|e| {
                Ok((
                    e.path().file_name().unwrap().to_string_lossy().to_string(),
                    Self::new(e.path())?,
                ))
            })
            .collect()
    }

    fn new(path: PathBuf) -> anyhow::Result<Self> {
        let open_file = std::fs::File::open(&path)?;
        let mmap = unsafe { MmapOptions::new().map(&open_file).unwrap() };
        let mmap: &'static Mmap = Box::leak(Box::new(mmap));
        let f = File::parse(mmap.deref() as &[u8])?;
        let file: &'static File<'static> = Box::leak(Box::new(f));

        // Set up the data structures
        let mut sym_tab = HashMap::<&'static str, RelocatedSymbol<'static>>::new();
        let mut parents = HashMap::<&'static str, HashSet<&'static str>>::new();

        // Build the symbol table
        for sect in file.sections() {
            for r in acc_symbols(&file, sect.index()) {
                sym_tab.insert(r.name, r);
            }
        }

        // Create a map of address -> symbol so we can resolve the section of a symbol
        let local_defs = file
            .symbols()
            .filter(|s| s.is_definition())
            .map(|s| (s.address(), s.name().unwrap()))
            .collect::<BTreeMap<_, _>>();

        // Build the call graph by walking the relocations
        // We keep track of what calls whata
        for (sym_name, sym) in sym_tab.iter() {
            let sym_section = file.section_by_index(sym.section).unwrap();
            let sym_data = sym_section.data().unwrap();

            for (addr, reloc) in sym.relocations.iter() {
                let target = match symbol_name_of_relo(file, reloc.target()) {
                    Some(name) => name,
                    None => {
                        let addend = u64::from_le_bytes(
                            sym_data[*addr as usize..(*addr + 8) as usize]
                                .try_into()
                                .unwrap(),
                        );
                        local_defs.get(&addend).unwrap()
                    }
                };

                parents.entry(target).or_default().insert(sym_name);
            }
        }

        Ok(Self {
            path,
            open_file,
            mmap,
            file,
            parents,
        })
    }
}

/// A function with its relevant relocations to be used for masked comparisons
struct RelocatedSymbol<'a> {
    name: &'a str,
    /// offset within the section
    offset: usize,
    data: &'a [u8],
    relocations: &'a [(u64, ReadRelocation)],
    sym: object::Symbol<'a, 'a>,
    section: SectionIndex,
}

fn acc_symbols<'a>(new: &'a File<'a>, section_idx: SectionIndex) -> Vec<RelocatedSymbol<'a>> {
    let mut syms = vec![];

    let section = new.section_by_index(section_idx).unwrap();

    let sorted = new
        .symbols()
        .filter(|s| s.section_index() == Some(section_idx))
        .sorted_by(|a, b| {
            let addr = a.address().cmp(&b.address());
            if addr == Ordering::Equal {
                a.index().0.cmp(&b.index().0)
            } else {
                addr
            }
        })
        .collect::<Vec<_>>();

    // todo!!!!!! jon: don't leak this lol
    let relocations = section
        .relocations()
        .sorted_by(|a, b| a.0.cmp(&b.0).reverse())
        .collect::<Vec<_>>()
        .leak();

    let data = section.data().unwrap();

    // No symbols, no symbols,
    if sorted.is_empty() {
        println!("No symbols for section: {:?}", section.name());
        return vec![];
    }

    // The end of the currently analyzed function
    let mut func_end = section.size() as usize;

    // The idx into the relocation list that applies to this function. We'll march these
    let mut reloc_idx = 0;

    // Walk in reverse so we can use the text_length as the initial backstop and to match relocation order
    for sym in sorted.into_iter().rev() {
        let sym_offset = sym.address() - section.address();

        // Move the head/tail to include the sub-slice of the relocations that apply to this symbol
        let mut reloc_start = None;
        loop {
            // If we've reached the end of the relocations then we're done
            if reloc_idx == relocations.len() {
                break;
            }

            // relocations behind the symbol start don't apply
            if relocations[reloc_idx].0 < sym_offset {
                break;
            }

            // Set the head to the first relocation that applies
            if reloc_start.is_none() {
                reloc_start = Some(reloc_idx);
            }

            reloc_idx += 1;
        }

        // Identify the instructions that apply to this symbol
        let data = match reloc_start {
            Some(_start) => &data[sym_offset as usize..func_end],
            _ => &[],
        };

        // Identify the relocations that apply to this symbol
        let relocations = match reloc_start {
            Some(start) => &relocations[start..reloc_idx],
            None => &[],
        };

        syms.push(RelocatedSymbol {
            name: sym.name().unwrap(),
            sym,
            offset: sym_offset as usize,
            data,
            relocations,
            section: section_idx,
        });

        func_end = sym_offset as usize;
    }

    assert_eq!(reloc_idx, relocations.len());

    syms
}

/// Compare two sets of bytes, masking out the bytes that are not part of the symbol
/// This is so we can compare functions with different relocations
fn compare_masked<'a>(
    old: &impl Object<'a>,
    new: &impl Object<'a>,
    left: &RelocatedSymbol,
    right: &RelocatedSymbol,
) -> bool {
    // Make sure the relocations are the same length
    if left.relocations.len() != right.relocations.len() {
        return false;
    }

    // Make sure the data is the same length
    // If the size changed then the instructions are different (well, not necessarily, but enough)
    if left.data.len() != right.data.len() {
        return false;
    }

    // Make sure the names match
    if left.name != right.name {
        return false;
    }

    // We're going to walk from relocation target to target, but since there's no implicit target
    // to start with, we simply use the end of the data
    let mut last = left.data.len();

    // Ensure the relocations point to the same symbol
    // Data symbols are special ... todo
    //
    // relocations are in reverse order, so we can also compare the data as we go
    for x in 0..left.relocations.len() {
        // Grab the reloc
        let (l_addr, left_reloc): &(u64, ReadRelocation) = &left.relocations[x];
        let (_r_addr, right_reloc): &(u64, ReadRelocation) = &right.relocations[x];

        // The targets might not be same by index but should resolve to the same *name*
        let left_target: RelocationTarget = left_reloc.target();
        let right_target: RelocationTarget = right_reloc.target();

        // Use the name of the symbol to compare
        // todo: decide if it's internal vs external
        let left_name = symbol_name_of_relo(old, left_target);
        let right_name = symbol_name_of_relo(new, right_target);
        let (Some(left_name), Some(right_name)) = (left_name, right_name) else {
            continue;
        };

        // Make sure the names match
        // if the target is a locally defined symbol, then it might be the same
        // todo(jon): hash the masked contents
        if left_name != right_name {
            return false;
        }

        // Check the data
        // the slice is the end of the relocation to the start of the previous relocation
        let reloc_byte_size = (left_reloc.size() as usize) / 8;
        let start = *l_addr as usize - left.offset as usize + reloc_byte_size;

        // Some relocations target the same location
        // In these cases, we just continue since we just masked and checked them already
        if (*l_addr as usize - left.offset as usize) == last {
            continue;
        }

        debug_assert!(start <= last);
        debug_assert!(start <= left.data.len());

        if &left.data[start..last] != &right.data[start..last] {
            return false;
        }

        if left_reloc.flags() != right_reloc.flags() {
            return false;
        }

        // todo: more checking... the symbols might be local
        last = start - reloc_byte_size;
    }

    // And a final check to make sure the data is the same
    if left.data[..last] != right.data[..last] {
        return false;
    }

    true
}

fn symbol_name_of_relo<'a>(obj: &impl Object<'a>, target: RelocationTarget) -> Option<&'a str> {
    match target {
        RelocationTarget::Symbol(symbol_index) => Some(
            obj.symbol_by_index(symbol_index)
                .unwrap()
                .name_bytes()
                .unwrap()
                .to_utf8(),
        ),
        RelocationTarget::Section(_) => None,
        RelocationTarget::Absolute => None,
        _ => None,
    }
}

fn workspace_dir() -> PathBuf {
    "/Users/jonkelley/Development/Tinkering/ipbp".into()
}

trait ToUtf8<'a> {
    fn to_utf8(&self) -> &'a str;
}

impl<'a> ToUtf8<'a> for &'a [u8] {
    fn to_utf8(&self) -> &'a str {
        std::str::from_utf8(self).unwrap()
    }
}

/// Builds an object file that satisfies the imports
///
/// Creates stub functions that jump to known addresses in a target process.
///
/// .section __TEXT,__text
/// .globl __ZN4core3fmt3num52_$LT$impl$u20$core..fmt..Debug$u20$for$u20$usize$GT$3fmt17h4e710f94be547818E
/// .p2align 2
/// __ZN4core3fmt3num52_$LT$impl$u20$core..fmt..Debug$u20$for$u20$usize$GT$3fmt17h4e710f94be547818E:
///     // Load 64-bit address using immediate values
///     movz x9, #0xCDEF          // Bottom 16 bits
///     movk x9, #0x89AB, lsl #16 // Next 16 bits
///     movk x9, #0x4567, lsl #32 // Next 16 bits
///     movk x9, #0x0123, lsl #48 // Top 16 bits
///
///     // Branch to the loaded address
///     br x9
fn build_stub(
    format: BinaryFormat,
    architecture: Architecture,
    endian: Endianness,
    adrp_imports: HashMap<&str, u64>,
) -> Result<Vec<u8>> {
    use object::{
        write::{Object, Symbol, SymbolSection},
        SectionKind, SymbolFlags, SymbolKind, SymbolScope,
    };

    // Create a new ARM64 object file
    let mut obj = Object::new(format, architecture, endian);

    // Add a text section for our trampolines
    let text_section = obj.add_section(Vec::new(), ".text".into(), SectionKind::Text);

    for (name, addr) in adrp_imports {
        // Add the symbol
        obj.add_symbol(Symbol {
            name: name.into(),
            value: addr,
            size: 0,
            kind: SymbolKind::Text,
            scope: SymbolScope::Dynamic,
            weak: false,
            section: SymbolSection::Section(text_section),
            flags: SymbolFlags::None,
        });
    }

    obj.write().context("Failed to write object file")
}

fn make_stub_file(
    proc_main_addr: u64,
    patch_target: PathBuf,
    adrp_imports: HashSet<&str>,
) -> Vec<u8> {
    let data = fs::read(&patch_target).unwrap();
    let old = File::parse(&data as &[u8]).unwrap();
    let main_sym = old.symbol_by_name_bytes(b"_main").unwrap();
    let aslr_offset = proc_main_addr - main_sym.address();
    let addressed = old
        .symbols()
        .filter_map(|sym| {
            adrp_imports
                .get(sym.name().ok()?)
                .copied()
                .map(|o| (o, sym.address() + aslr_offset))
        })
        .collect::<HashMap<_, _>>();

    build_stub(
        old.format(),
        old.architecture(),
        old.endianness(),
        addressed,
    )
    .unwrap()
}

#[test]
fn parse_wasm_and_print_globals() {
    // let bytes = include_bytes!("/Users/jonkelley/Development/dioxus/target/dx/fullstack-hello-world-example/debug/web/public/wasm/libfullstack-hello-world-example-patch-1744937420042.wasm");
    // let module = walrus::Module::from_buffer(bytes).unwrap();
    // let globals = module.globals.iter().collect::<Vec<_>>();
    // for g in globals {
    //     println!("{:?}: {:?}", g.name, g.kind);
    // }
}

const BADNAME: &str = "_ZN4core3ptr68drop_in_place$LT$alloc..boxed..Box$LT$dyn$u20$core..any..Any$GT$$GT$17hcd167959be12f848E";

#[test]
fn parse_wasm_and_print_globals2() {
    let bytes = &[];
    // let bytes = include_bytes!("/Users/jonkelley/Development/dioxus/target/dx/fullstack-hello-world-example/debug/web/public/wasm/fullstack-hello-world-example_bg.wasm");
    let module = walrus::Module::from_buffer(bytes).unwrap();
    let func = module.funcs.by_name(BADNAME).unwrap();

    let data = parse_bytes_to_data_segment(bytes).unwrap();
    let ifuncs = collect_func_ifuncs(&module, &data);

    // 55874
    println!("there are {} ifuncs", ifuncs.len());

    let ifunc = ifuncs.get(BADNAME).unwrap();

    println!("ifunc entry: {:?}", ifunc);
}

#[test]
fn hoists() {
    let bytes = include_bytes!("/Users/jonkelley/Development/dioxus/target/wasm32-unknown-unknown/wasm-dev/fullstack-hello-world-example.wasm");
    let out_bytes = prepare_wasm_base_module(bytes).unwrap();

    let out = walrus::Module::from_buffer(&out_bytes).unwrap();
    let syms = parse_bytes_to_data_segment(&out_bytes).unwrap();
    let ifuncs = collect_func_ifuncs(&out, &syms);

    // 57001
    println!("there are {} ifuncs", ifuncs.len());

    let ifunc = ifuncs.get(BADNAME).unwrap();
    println!("ifunc entry: {:?}", ifunc);
}

#[test]
fn delta() {
    let pre_bytes = include_bytes!("/Users/jonkelley/Development/dioxus/target/wasm32-unknown-unknown/wasm-dev/fullstack-hello-world-example.wasm");
    // let prepared_out_bytes = prepare_wasm_base_module(pre_bytes).unwrap();

    let prepared_out_bytes = pre_bytes;

    let pre_module = walrus::Module::from_buffer(prepared_out_bytes).unwrap();
    // let pre_module = walrus::Module::from_buffer(&prepared_out_bytes).unwrap();
    let pre_syms = parse_bytes_to_data_segment(prepared_out_bytes).unwrap();
    let pre_ifuncs = collect_func_ifuncs(&pre_module, &pre_syms);
    let pre_name_map = fn_name_map(&pre_syms);

    // let bg_bytes = include_bytes!("/Users/jonkelley/Development/dioxus/target/dx/fullstack-hello-world-example/debug/web/public/wasm/fullstack-hello-world-example_bg.wasm");
    let bg_bytes = &[];
    let bg_module = walrus::Module::from_buffer(bg_bytes).unwrap();
    let bg_syms = parse_bytes_to_data_segment(bg_bytes).unwrap();
    let bg_ifuncs = collect_func_ifuncs(&bg_module, &bg_syms);
    let bg_name_map = fn_name_map(&bg_syms);

    let pre_funcs = pre_module
        .funcs
        .iter()
        .map(|f| (f.id().index(), f))
        .collect::<HashMap<_, _>>();

    let bg_funcs = bg_module
        .funcs
        .iter()
        .map(|f| (f.id().index(), f))
        .collect::<HashMap<_, _>>();

    // for p in pre_ifuncs.iter() {
    //     if !bg_ifuncs.contains_key(*p.0) {
    //         println!("pre->: {:?}", p);

    //         // let f = pre_funcs[&(*p.1 as usize)];
    //         // println!("pre func: {:?}", f.name);
    //     }
    // }

    let mut bad = 0;
    for p in pre_name_map.iter() {
        // if !bg_name_map.contains_key(*p.0) {
        //     println!("pre->: {:?}", p);
        // }

        let pre = pre_funcs.get(&(p.1 .0 as usize)).unwrap();
        if pre.name.as_deref() != Some(*p.0) {
            // println!("pre->: {:?} -> {:?}", pre.name, p.0);
            bad += 1;
        }
    }
    println!("bad: {bad}");
    println!("total: {}", bg_name_map.len());

    let mut bad = 0;
    for p in bg_name_map.iter() {
        // if !bg_name_map.contains_key(*p.0) {
        //     println!("pre->: {:?}", p);
        // }

        let bg = bg_funcs.get(&(p.1 .0 as usize)).unwrap();
        if bg.name.as_deref() != Some(*p.0) {
            // println!("pre->: {:?} -> {:?}", pre.name, p.0);
            bad += 1;
        }
    }

    println!("bad: {bad}");
    println!("total: {}", bg_name_map.len());

    // for p in bg_ifuncs {
    //     if !pre_ifuncs.contains_key(p.0) {
    //         println!("bg->: {:?}", p);
    //     }
    // }

    // // 57001
    // println!("there are {} ifuncs", ifuncs.len());

    // let ifunc = ifuncs.get(BADNAME).unwrap();
    // println!("ifunc entry: {:?}", ifunc);
}
