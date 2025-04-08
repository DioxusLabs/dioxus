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

pub mod lift;
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

    for (new_name, new_addr) in new_name_to_addr {
        if let Some(old_addr) = old_name_to_addr.get(new_name) {
            map.insert(*old_addr, new_addr);
        }
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
        got: Default::default(),
        old_base_address,
        new_base_address,
        aslr_reference,
    })
}

/// In the web, our patchable functions are actually ifuncs
///
/// We need to line up the ifuncs from the main module to the ifuncs in the patch.
fn create_wasm_jump_table(original: &Path, patch: &Path) -> anyhow::Result<JumpTable> {
    tracing::info!("jumping {} to {}", original.display(), patch.display());
    let obj1_bytes = fs::read(original).context("Could not read original file")?;
    let obj2_bytes = fs::read(patch).context("Could not read patch file")?;

    let mod_old = walrus::Module::from_buffer(&obj1_bytes)?;
    let mod_new = walrus::Module::from_buffer(&obj2_bytes)?;

    let name_to_ifunc_old = collect_func_ifuncs(&mod_old);
    let name_to_ifunc_new = collect_func_ifuncs(&mod_new);

    let mut map = AddressMap::default();
    for (name, idx) in name_to_ifunc_new {
        if let Some(old_idx) = name_to_ifunc_old.get(name) {
            // tracing::info!("Mapping {name} from {old_idx} to {idx}");
            map.insert(*old_idx as u64, idx as u64);
        }
    }

    tracing::info!("Jump table: {:?}", map);
    for data in mod_new.data.iter() {
        tracing::info!("Data: {:?} - {:?}", data.name, data.kind);
    }

    for global in mod_new.globals.iter() {
        tracing::info!("Global: {:?} - {:?}", global.name, global.kind);
    }
    for el in mod_new.elements.iter() {
        tracing::info!("Elemenet: {:?} - {:?}", el.name, el.kind);
    }

    Ok(JumpTable {
        map,
        got: Default::default(),
        lib: patch.to_path_buf(),
        aslr_reference: 0,
        old_base_address: 0,
        new_base_address: 0,
    })
}

fn collect_func_ifuncs(mod_new: &Module) -> HashMap<&str, i32> {
    tracing::info!("Collecting ifuncs from module");
    let mut name_to_ifunc_index = HashMap::new();

    for el in mod_new.elements.iter() {
        let ElementKind::Active { table, offset } = &el.kind else {
            continue;
        };
        let offset = match offset {
            walrus::ConstExpr::Value(value) => match value {
                walrus::ir::Value::I32(idx) => *idx,
                walrus::ir::Value::I64(_) => todo!(),
                walrus::ir::Value::F32(_) => todo!(),
                walrus::ir::Value::F64(_) => todo!(),
                walrus::ir::Value::V128(_) => todo!(),
            },
            walrus::ConstExpr::Global(id) => {
                let global = mod_new.globals.get(*id);
                tracing::info!("refercning global {:?}", global);
                continue;
            }
            walrus::ConstExpr::RefNull(ref_type) => todo!(),
            walrus::ConstExpr::RefFunc(id) => todo!(),
        };

        match &el.items {
            ElementItems::Functions(ids) => {
                for (idx, id) in ids.iter().enumerate() {
                    let func = mod_new.funcs.get(*id);
                    let name = func.name.as_ref().unwrap();
                    name_to_ifunc_index.insert(name.as_str(), offset + idx as i32);
                }
            }
            ElementItems::Expressions(ref_type, const_exprs) => {
                panic!("Unsupported element kind: {:?}", ref_type);
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
            aslr_reference
                - symbol_table
                    .get("_aslr_reference")
                    .unwrap_or_else(|| {
                        symbol_table
                            .get("aslr_reference")
                            .expect("failed to find aslr_reference")
                    })
                    .address()
        }
    };

    if triple.architecture == target_lexicon::Architecture::Wasm32 {
        return Ok(vec![]);
    }

    // we need to assemble a PLT/GOT so direct calls to the patch symbols work
    // for each symbol we either write the address directly (as a symbol) or create a PLT/GOT entry
    let text_section = obj.section_id(StandardSection::Text);
    for name in undefined_symbols {
        if let Some(sym) = symbol_table.get(name.as_str()) {
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
        } else {
            tracing::error!("Symbol not found: {}", name);
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

/// The incoming module is expecting to initialize its functions at address 1.
///
/// Since the user can reload the page, killing our ASLR reference, we need to make the patch itself
/// relocatable.
///
/// This involves making the ifunc initializers
pub fn move_func_initiailizers(bytes: &[u8]) -> Result<(Vec<u8>, HashMap<String, u64>)> {
    let mut module = walrus::Module::from_buffer(bytes)?;

    let mut offsets = vec![];

    let (ifunc_global, _) =
        module.add_import_global("env", "__IFUNC_OFFSET", ValType::I32, false, false);

    let (ro_data, _) =
        module.add_import_global("env", "__RO_DATA_OFFSET", ValType::I32, false, false);

    let (bss_data, _) =
        module.add_import_global("env", "__BSS_DATA_OFFSET", ValType::I32, false, false);

    let table = module.tables.iter_mut().next().unwrap();
    let segments = table.elem_segments.clone();

    for seg in segments {
        if let ElementKind::Active { table, offset } = &mut module.elements.get_mut(seg).kind {
            tracing::info!("Updating active Segment: {:?}", seg);
            *offset = walrus::ConstExpr::Global(ifunc_global);
        }
    }

    let datas = module.data.iter().map(|f| f.id()).collect::<Vec<_>>();
    for data in datas {
        let data = module.data.get_mut(data);
        if let walrus::DataKind::Active { offset, .. } = &mut data.kind {
            let orig_offset = match &offset {
                walrus::ConstExpr::Value(value) => match value {
                    walrus::ir::Value::I32(val) => *val,
                    _ => continue,
                },
                _ => continue,
            };

            match data.name.as_deref() {
                Some(".bss") => {
                    tracing::info!("Updating bss segment: {:?}", data.name);
                    *offset = walrus::ConstExpr::Global(bss_data);
                    offsets.push(("__BSS_DATA_OFFSET".to_string(), orig_offset as u64));
                }
                Some(".rodata") => {
                    tracing::info!("Updating rodata segment: {:?}", data.name);
                    *offset = walrus::ConstExpr::Global(ro_data);
                    offsets.push(("__RO_DATA_OFFSET".to_string(), orig_offset as u64));
                }
                _ => continue,
            }
        }
    }

    // Sort by offset order
    offsets.sort_by(|a, b| a.1.cmp(&b.1));

    // And then normalize to the first offset
    let first_offset = offsets.first().map(|f| f.1).unwrap_or(0);
    let offsets = offsets
        .iter()
        .map(|(name, offset)| (name.clone(), offset - first_offset))
        .collect::<HashMap<_, _>>();

    Ok((module.emit_wasm(), offsets))
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

fn get_ifunc_table_length(bytes: &[u8]) -> usize {
    let module = walrus::Module::from_buffer(bytes).unwrap();
    module
        .tables
        .iter()
        .map(|table| table.elem_segments.iter())
        .flatten()
        .map(|segment| match &module.elements.get(*segment).items {
            ElementItems::Functions(ids) => ids.len(),
            ElementItems::Expressions(ref_type, const_exprs) => const_exprs.len(),
        })
        // .map(|table| table.elem_segments.len())
        .max()
        .unwrap_or(1)
}

#[test]
fn test_prepare_base_module() {
    prepare_wasm_base_module(include_bytes!("../../data/wasm-1/pre-bindgen.wasm"));
}

#[test]
fn ensure_matching() -> Result<()> {
    let patch = include_bytes!("../../data/wasm-1/patch.wasm");
    let post_bind = include_bytes!("../../data/wasm-1/post-bindgen.wasm");

    let patch_module = walrus::Module::from_buffer(patch).unwrap();
    let post_bindgen_module = walrus::Module::from_buffer(post_bind).unwrap();

    for import in patch_module.imports.iter() {
        println!("Import: {}", import.name);
    }

    Ok(())
}

#[test]
fn print_data_sections() {
    let base: PathBuf = "/Users/jonkelley/Development/dioxus/packages/subsecond/subsecond-harness/static/main_bg.wasm".into();
    let patch: PathBuf = "/Users/jonkelley/Development/dioxus/packages/subsecond/subsecond-harness/static/patch-1742923392809.wasm".into();
    let base_bytes = fs::read(&base).unwrap();
    let patch_bytes = fs::read(&patch).unwrap();

    let base_module = Module::from_buffer(&base_bytes).unwrap();
    let raw_data = parse_bytes_to_data_segment(&base_bytes).unwrap();

    let base_data_syms: HashMap<&str, _> = raw_data
        .iter()
        .flat_map(|f| match f {
            SymbolInfo::Data {
                flags,
                name,
                symbol,
            } => Some((*name, symbol)),
            SymbolInfo::Func { flags, index, name } => None,
            SymbolInfo::Global { flags, index, name } => None,
            SymbolInfo::Section { flags, section } => None,
            SymbolInfo::Event { flags, index, name } => None,
            SymbolInfo::Table { flags, index, name } => None,
        })
        .collect();

    let patch_data_syms: HashMap<&str, _> = raw_data
        .iter()
        .flat_map(|f| match f {
            SymbolInfo::Data {
                flags,
                name,
                symbol,
            } => match symbol {
                Some(sym) => Some((*name, symbol)),
                None => Some((*name, symbol)),
            },
            SymbolInfo::Func { flags, index, name } => None,
            SymbolInfo::Global { flags, index, name } => None,
            SymbolInfo::Section { flags, section } => None,
            SymbolInfo::Event { flags, index, name } => None,
            SymbolInfo::Table { flags, index, name } => None,
        })
        .collect();

    println!("undefined patch data: {:?}", patch_data_syms);
    for (sym, _def) in patch_data_syms {
        if base_data_syms.contains_key(sym) {
            if sym.contains("signal") || sym.contains("Signal") {
                println!("zero-init sym: {sym}");
            }
        }
    }
}
