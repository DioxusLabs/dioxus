use anyhow::Context;
use itertools::Itertools;
use object::{
    macho::{self},
    read::File,
    write::{MachOBuildVersion, StandardSection, Symbol, SymbolSection},
    Endianness, Object, ObjectSymbol, SymbolKind, SymbolScope,
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
};
use std::{
    fs,
    ops::{Deref, Range},
    path::PathBuf,
    sync::{Arc, RwLock},
};
use subsecond_types::*;
use target_lexicon::{OperatingSystem, Triple};
use thiserror::Error;
use walrus::{
    ConstExpr, ElementItems, ElementKind, ExportItem, FunctionId, FunctionKind, ImportKind, Module,
    ModuleConfig,
};
use wasmparser::{
    BinaryReader, BinaryReaderError, Linking, LinkingSectionReader, Payload, SymbolInfo,
};

type Result<T, E = PatchError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum PatchError {
    #[error("Failed to read file: {0}")]
    ReadFs(#[from] std::io::Error),

    #[error("Missing symbols in the object file {symbols:?}")]
    MissingSymbols { symbols: Vec<String> },

    #[error("Failed to parse wasm section: {0}")]
    ParseSection(#[from] wasmparser::BinaryReaderError),

    #[error("Failed to parse object file, {0}")]
    ParseObjectFile(#[from] object::read::Error),

    #[error("Failed to write object file: {0}")]
    WriteObjectFIle(#[from] object::write::Error),

    #[error("Failed to emit module: {0}")]
    RuntimeError(#[from] anyhow::Error),

    #[error("{0}")]
    InvalidModule(String),
}

pub fn create_jump_table(original: &Path, patch: &Path, triple: &Triple) -> Result<JumpTable> {
    // WASM needs its own path since the object crate leaves quite a few of the methods unimplemented
    if triple.architecture == target_lexicon::Architecture::Wasm32 {
        return create_wasm_jump_table(original, patch);
    }

    let obj1_bytes = fs::read(original)?;
    let obj2_bytes = fs::read(patch)?;
    let obj1 = File::parse(&obj1_bytes as &[u8])?;
    let obj2 = File::parse(&obj2_bytes as &[u8])?;

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

    let new_base_address = match triple.operating_system {
        // The symbol in the symtab is called "_main" but in the dysymtab it is called "main"
        OperatingSystem::MacOSX(_) | OperatingSystem::Darwin(_) | OperatingSystem::IOS(_) => {
            *new_name_to_addr.get("_main").unwrap()
        }

        // No distincation between the two on these platforms
        OperatingSystem::Freebsd
        | OperatingSystem::Openbsd
        | OperatingSystem::Linux
        | OperatingSystem::Windows => *new_name_to_addr.get("main").unwrap(),

        // On wasm, it doesn't matter what the address is since we don't use ASLR
        _ => 0,
    };

    let aslr_reference = *old_name_to_addr.get("_aslr_reference").unwrap_or_else(|| {
        old_name_to_addr
            .get("aslr_reference")
            .expect("failed to find aslr_reference")
    });

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
///
/// According to the dylink spec, there will be two sets of entries:
///
/// - got.func: functions in the indirect function table
/// - got.mem: data objects in the data segments
///
/// It doesn't seem like we can compile the base module to export these, sadly, so we're going
/// to manually satisfy them here, removing their need to be imported.
///
/// https://github.com/WebAssembly/tool-conventions/blob/main/DynamicLinking.md
fn create_wasm_jump_table(original: &Path, patch: &Path) -> Result<JumpTable> {
    let old_bytes = fs::read(original).context("Could not read original file")?;
    let new_bytes = fs::read(patch).context("Could not read patch file")?;

    let old = walrus::Module::from_buffer(&old_bytes)?;
    let mut new = walrus::Module::from_buffer(&new_bytes)?;

    let old_raw_data = parse_bytes_to_data_segment(&old_bytes)
        .context("Failed to parse old bytes data segment")?;
    let new_raw_data = parse_bytes_to_data_segment(&new_bytes)
        .context("Failed to parse new bytes data segment")?;

    let name_to_ifunc_old = collect_func_ifuncs(&old);

    if old_raw_data.symbols.is_empty() {
        tracing::warn!("No debug symbols in the WASM output. Make sure to set `opt-level = 0` for hotpatching to work properly.");
        return Err(PatchError::MissingSymbols { symbols: vec![] });
    }

    // Do a quick scan to see if the symbols in the wasm-bindgen table have changed at all
    // We currently don't support updating them since we'd somehow need to merge the glue code together
    ensure_wasm_bindgen_unchanged(&old, &new, &old_raw_data, &new_raw_data)?;

    let mut mems = vec![];
    let mut funcs = vec![];

    // Collect all the GOT entries from the new module.
    for t in new.imports.iter() {
        match t.module.as_str() {
            "GOT.func" => {
                let Some(entry) = name_to_ifunc_old.get(t.name.as_str()).cloned() else {
                    let exists = old.exports.get_func(t.name.as_str());
                    return Err(PatchError::InvalidModule(format!("Expected to find GOT.func entry in ifunc table but it was missing: {} -> {exists:?}\nDid all symbols make it into the static lib?", t.name.as_str())));
                };
                funcs.push((t.id(), entry));
            }
            "GOT.mem" => mems.push(t.id()),
            _ => {}
        }
    }

    // We need to satisfy the GOT.func imports of this side module. The GOT imports come from the wasm-ld
    // implementation of the dynamic linking spec
    //
    // https://github.com/WebAssembly/tool-conventions/blob/main/DynamicLinking.md#imports
    //
    // Most importantly, these functions are functions meant to be called indirectly. In normal wasm
    // code generation, only functions that Rust code references via pointers are given a slot in
    // the indirection function table. The optimization here traditionally meaning that if a function
    // can be called directly, then it doesn't need to be referenced indirectly and potentially inlined
    // or dissolved during LTO.
    //
    // In our "fat build" setup, we aggregated all symbols from dependencies into a `dependencies.ar` file.
    // By promoting these functions to the dynamic scope, we also prevent their inlining because the
    // linker can still expect some form of interposition to happen, requiring the symbol *actually*
    // exists.
    //
    // Our technique here takes advantage of that and the [`prepare_wasm_base_module`] function promotes
    // every possible function to the indirect function table. This means that the GOT imports that
    // `relocation-model=pic` synthesizes can reference the functions via the indirect function table
    // even if they are not normally synthesized in regular wasm code generation.
    //
    // Normally, the dynaic linker setup would resolve GOT.func against the same GOT.func export in
    // the main module, but we don't have that. Instead, we simply re-parse the main module, aggregate
    // its ifunc table, and then resolve directly to the index in that table.
    for (import_id, ifunc_index) in funcs {
        let ImportKind::Global(id) = new.imports.get(import_id).kind else {
            return Err(PatchError::InvalidModule(
                "Expected GOT.func import to be a global".into(),
            ));
        };

        // "satisfying" the import means removing it from the import table and replacing its target
        // value with a local global.
        new.imports.delete(import_id);
        new.globals.get_mut(id).kind =
            walrus::GlobalKind::Local(ConstExpr::Value(walrus::ir::Value::I32(ifunc_index)));
    }

    // We need to satisfy the GOT.mem imports of this side module. The GOT.mem imports come from the wasm-ld
    // implementation of the dynamic linking spec
    //
    // https://github.com/WebAssembly/tool-conventions/blob/main/DynamicLinking.md#imports
    //
    // Unlike the ifunc table, the GOT.mem imports do not need any additional post-processing of the
    // base module to satisfy. Since our patching approach works but leveraging the experimental dynamic
    // PIC support in rustc[wasm] and wasm-ld, we are using the GOT.mem imports as a way of identifying
    // data segments that are present in the base module.
    //
    // Normally, the dynamic linker would synthesize corresponding GOT.mem exports in the main module,
    // but since we're patching on-the-fly, this table will always be out-of-date.
    //
    // Instead, we use the symbol table from the base module to find the corresponding data symbols
    // and then resolve the offset of the data segment in the main module. Using the symbol table
    // can be somewhat finicky if the user compiled the code with a high-enough opt level that nukes
    // the names of the data segments, but otherwise this system works well.
    //
    // We simply use the name of the import as a key into the symbol table and then its offset into
    // its data segment as the value within the global.
    for mem in mems {
        let import = new.imports.get(mem);
        let data_symbol_idx = *old_raw_data
            .data_symbol_map
            .get(import.name.as_str())
            .with_context(|| {
                format!("Failed to find GOT.mem import by its name: {}", import.name)
            })?;
        let data_symbol = old_raw_data
            .data_symbols
            .get(&data_symbol_idx)
            .context("Failed to find data symbol by its index")?;
        let data = old
            .data
            .iter()
            .nth(data_symbol.which_data_segment)
            .context("Missing data segment in the main module")?;

        let offset = match data.kind {
            walrus::DataKind::Active {
                offset: ConstExpr::Value(walrus::ir::Value::I32(idx)),
                ..
            } => idx,
            walrus::DataKind::Active {
                offset: ConstExpr::Value(walrus::ir::Value::I64(idx)),
                ..
            } => idx as i32,
            _ => {
                return Err(PatchError::InvalidModule(format!(
                    "Data segment of invalid table: {:?}",
                    data.kind
                )));
            }
        };

        let ImportKind::Global(global_id) = import.kind else {
            return Err(PatchError::InvalidModule(
                "Expected GOT.mem import to be a global".to_string(),
            ));
        };

        // "satisfying" the import means removing it from the import table and replacing its target
        // value with a local global.
        new.imports.delete(mem);
        new.globals.get_mut(global_id).kind = walrus::GlobalKind::Local(ConstExpr::Value(
            walrus::ir::Value::I32(offset + data_symbol.segment_offset as i32),
        ));
    }

    // Update the wasm module on the filesystem to use the newly lifted version
    let lib = patch.to_path_buf();
    std::fs::write(&lib, new.emit_wasm())?;

    // And now assemble the jump table by mapping the old ifunc table to the new one, by name
    //
    // The ifunc_count will be passed to the dynamic loader so it can allocate the right amount of space
    // in the indirect function table when loading the patch.
    let name_to_ifunc_new = collect_func_ifuncs(&new);
    let ifunc_count = name_to_ifunc_new.len() as u64;
    let mut map = AddressMap::default();
    for (name, idx) in name_to_ifunc_new.iter() {
        if let Some(old_idx) = name_to_ifunc_old.get(name) {
            map.insert(*old_idx as u64, *idx as u64);
        }
    }

    tracing::trace!("Jump table: {:#?}", map);

    Ok(JumpTable {
        map,
        lib,
        aslr_reference: 0,
        new_base_address: 0,
        ifunc_count,
    })
}

fn ensure_wasm_bindgen_unchanged(
    _old: &Module,
    _new: &Module,
    _old_symbols: &RawDataSection,
    _new_symbols: &RawDataSection,
) -> Result<()> {
    // todo: implement diffing
    // for sym in new_symbols.symbols.iter() {
    //     match sym {
    //         SymbolInfo::Func { flags, index, name } => {}
    //         SymbolInfo::Data {
    //             flags,
    //             name,
    //             symbol,
    //         } => {}
    //         _ => {}
    //     }
    // }

    Ok(())
}

fn collect_func_ifuncs<'a>(
    m: &'a Module,
    raw: &RawDataSection<'a>,
    ids_to_fns: &Vec<FunctionId>,
) -> HashMap<&'a str, i32> {
    let mut func_to_offset = HashMap::new();

    for el in m.elements.iter() {
        let ElementKind::Active { offset, .. } = &el.kind else {
            continue;
        };

        let offset = match offset {
            // Handle explicit offsets
            ConstExpr::Value(value) => match value {
                walrus::ir::Value::I32(idx) => *idx,
                walrus::ir::Value::I64(idx) => *idx as i32,
                _ => continue,
            },

            // Globals are usually imports and thus don't add a specific offset
            // ie the ifunc table is offset by a global, so we don't need to push the offset out
            ConstExpr::Global(_) => 0,
            _ => continue,
        };

        match &el.items {
            ElementItems::Functions(ids) => {
                for (idx, id) in ids.iter().enumerate() {
                    func_to_offset.insert(*id, offset + idx as i32);
                }
            }
            ElementItems::Expressions(_ref_type, _const_exprs) => {
                // todo - do we need to handle these?
            }
        }
    }

    let mut offsets = HashMap::new();

    for sym in raw.symbols.iter() {
        if let SymbolInfo::Func { index, name, .. } = sym {
            let id = ids_to_fns[*index as usize];
            let Some(offset) = func_to_offset.get(&id) else {
                continue;
            };
            let Some(name) = name else {
                continue;
            };
            offsets.insert(*name, *offset as i32);
        }
    }

    offsets
}

/// Resolve the undefined symbols in the incrementals against the original binary, returning an object
/// file that can be linked along the incrementals.
///
/// This makes it possible to dlopen the resulting object file and use the original binary's symbols
/// bypassing the dynamic linker.
///
/// This is very similar to malware :) but it's not!
pub fn create_undefined_symbol_stub(
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
        let bytes = fs::read(path).with_context(|| format!("failed to read {:?}", path))?;
        let file = File::parse(bytes.deref() as &[u8])?;
        for symbol in file.symbols() {
            if symbol.is_undefined() {
                undefined_symbols.insert(symbol.name()?.to_string());
            } else if symbol.is_global() {
                defined_symbols.insert(symbol.name()?.to_string());
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
    #[allow(clippy::identity_op)]
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
    let bytes = fs::read(source_path)?;
    let source = File::parse(bytes.deref() as &[u8]).context("Failed to parse")?;
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
            continue;
        }

        let name_offset = match triple.operating_system {
            target_lexicon::OperatingSystem::Darwin(_) => 1,
            target_lexicon::OperatingSystem::IOS(_) => 1,
            _ => 0,
        };

        let abs_addr = sym.address() + aslr_offset;

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
    let ParsedModule {
        mut module,
        ids,
        symbols,
        ..
    } = parse_module_with_ids(bytes)?;

    // Due to monomorphizations, functions will get merged and multiple names will point to the same function.
    // Walrus loses this information, so we need to manually parse the names table to get the indices
    // and names of these functions.
    //
    // Unfortunately, the indices it gives us ARE NOT VALID.
    // We need to work around it by using the FunctionId from the module as a link between the merged function names.
    let ifunc_map = collect_func_ifuncs(&module, &symbols, &ids);
    let ifuncs = module
        .funcs
        .iter()
        .filter_map(|f| ifunc_map.get(f.name.as_deref()?).map(|_| f.id()))
        .collect::<HashSet<_>>();

    let ifunc_table_initializer = module
        .elements
        .iter()
        .last()
        .context("Missing ifunc table")?
        .id();

    let mut already_exported = module
        .exports
        .iter()
        .filter(|f| matches!(f.item, ExportItem::Function(_)))
        .map(|exp| exp.name.clone())
        .collect::<HashSet<_>>();

    let mut make_indirect = vec![];
    for (name, index) in symbols.code_symbol_map.iter() {
        let func = module.funcs.get(ids[*index]);

        if let FunctionKind::Local(_local) = &func.kind {
            if !already_exported.contains(*name) && !name_is_bindgen_symbol(name) {
                module.exports.add(name, func.id());
                already_exported.insert(name.to_string());
            }

            if !ifuncs.contains(&func.id()) && !name_is_bindgen_symbol(name) {
                make_indirect.push(func.id());
            }
        }
    }

    tracing::trace!("Hoisting {} functions", make_indirect.len());
    let seg = module.elements.get_mut(ifunc_table_initializer);
    let make_indirect_count = make_indirect.len() as u64;
    if let ElementItems::Functions(ids) = &mut seg.items {
        for func in make_indirect {
            ids.push(func);
        }
    };

    if let ElementKind::Active { table, .. } = seg.kind {
        let table = module.tables.get_mut(table);
        table.initial += make_indirect_count;
        if let Some(max) = table.maximum {
            table.maximum = Some(max + make_indirect_count);
        }
    }

    Ok(module.emit_wasm())
}

fn name_is_bindgen_symbol(name: &str) -> bool {
    name.starts_with("__wbindgen_describe_")
        || name.contains("wasm_bindgen..describe..WasmDescribe")
        || name.contains("wasm_bindgen..closure..WasmClosure$GT$8describe")
        || name.contains("wasm_bindgen7closure16Closure$LT$T$GT$4wrap8describe")
        || name.contains("__wbindgen_describe_closure")
        || name.contains("__wbindgen_externref_xform")
}

/// Manually parse the data section from a wasm module
///
/// We need to do this for data symbols because walrus doesn't provide the right range and offset
/// information for data segments. Fortunately, it provides it for code sections, so we only need to
/// do a small amount extra of parsing here.
fn parse_bytes_to_data_segment(bytes: &[u8]) -> Result<RawDataSection> {
    let parser = wasmparser::Parser::new(0);
    let mut parser = parser.parse_all(bytes);
    let mut segments = vec![];
    let mut data_range = 0..0;
    let mut symbols = vec![];

    // Process the payloads in the raw wasm file so we can extract the specific sections we need
    while let Some(Ok(payload)) = parser.next() {
        match payload {
            Payload::DataSection(section) => {
                data_range = section.range();
                segments = section
                    .into_iter()
                    .collect::<Result<Vec<_>, BinaryReaderError>>()?
            }
            Payload::CustomSection(section) if section.name() == "linking" => {
                let reader = BinaryReader::new(section.data(), 0);
                let reader = LinkingSectionReader::new(reader)?;
                for subsection in reader.subsections() {
                    if let Linking::SymbolTable(map) = subsection? {
                        symbols = map.into_iter().collect::<Result<Vec<_>, _>>()?;
                    }
                }
            }
            Payload::CustomSection(section) => {
                tracing::trace!("Skipping Custom section: {:?}", section.name());
            }
            _ => {}
        }
    }

    // Accumulate the data symbols into a btreemap for later use
    let mut data_symbols = BTreeMap::new();
    let mut data_symbol_map = HashMap::new();
    let mut code_symbol_map = BTreeMap::new();
    for (index, symbol) in symbols.iter().enumerate() {
        if let SymbolInfo::Func { name, index, .. } = symbol {
            if let Some(name) = name {
                code_symbol_map.insert(*name, *index as usize);
            }
            continue;
        }

        let SymbolInfo::Data {
            symbol: Some(symbol),
            name,
            ..
        } = symbol
        else {
            continue;
        };

        data_symbol_map.insert(*name, index);

        if symbol.size == 0 {
            continue;
        }

        let data_segment = segments
            .get(symbol.index as usize)
            .context("Failed to find data segment")?;
        let offset: usize =
            data_segment.range.end - data_segment.data.len() + (symbol.offset as usize);
        let range = offset..(offset + symbol.size as usize);

        data_symbols.insert(
            index,
            DataSymbol {
                _index: index,
                _range: range,
                segment_offset: symbol.offset as usize,
                _symbol_size: symbol.size as usize,
                which_data_segment: symbol.index as usize,
            },
        );
    }

    Ok(RawDataSection {
        _data_range: data_range,
        symbols,
        data_symbols,
        data_symbol_map,
        code_symbol_map,
    })
}

struct RawDataSection<'a> {
    _data_range: Range<usize>,
    symbols: Vec<SymbolInfo<'a>>,
    code_symbol_map: BTreeMap<&'a str, usize>,
    data_symbols: BTreeMap<usize, DataSymbol>,
    data_symbol_map: HashMap<&'a str, usize>,
}

#[derive(Debug)]
struct DataSymbol {
    _index: usize,
    _range: Range<usize>,
    segment_offset: usize,
    _symbol_size: usize,
    which_data_segment: usize,
}

struct ParsedModule<'a> {
    module: Module,
    ids: Vec<FunctionId>,
    fns_to_ids: HashMap<FunctionId, usize>,
    symbols: RawDataSection<'a>,
}

/// Parse a module and return the mapping of index to FunctionID.
/// We'll use this mapping to remap ModuleIDs
fn parse_module_with_ids(bindgened: &[u8]) -> Result<ParsedModule> {
    let ids = Arc::new(RwLock::new(Vec::new()));
    let ids_ = ids.clone();
    let module = Module::from_buffer_with_config(
        bindgened,
        ModuleConfig::new().on_parse(move |_m, our_ids| {
            let mut ids = ids_.write().expect("No shared writers");
            let mut idx = 0;
            while let Ok(entry) = our_ids.get_func(idx) {
                ids.push(entry);
                idx += 1;
            }

            Ok(())
        }),
    )?;
    let mut ids_ = ids.write().expect("No shared writers");
    let mut ids = vec![];
    std::mem::swap(&mut ids, &mut *ids_);

    let mut fns_to_ids = HashMap::new();
    for (idx, id) in ids.iter().enumerate() {
        fns_to_ids.insert(*id, idx);
    }

    let symbols = parse_bytes_to_data_segment(bindgened).context("Failed to parse data segment")?;

    Ok(ParsedModule {
        module,
        ids,
        fns_to_ids,
        symbols,
    })
}

// #[test]
// fn compare_bindgen_sections() {
//     let bytes = include_bytes!("/Users/jonathankelley/Development/dioxus/target/wasm32-unknown-unknown/wasm-dev/fullstack-hello-world-example.wasm");
//     let (module, ids, _fns_to_ids) = parse_module_with_ids(bytes).unwrap();
//     let (sect, section) = module
//         .customs
//         .iter()
//         .find(|(id, f)| f.name() == "__wasm_bindgen_unstable")
//         .unwrap();
//     let data = section.data(&Default::default());

//     let syms = parse_bytes_to_data_segment(bytes).unwrap();
//     for s in syms.symbols.iter() {
//         match s {
//             SymbolInfo::Func { flags, index, name } => {
//                 if let Some(name) = name {
//                     if name_is_bindgen_symbol(name) {
//                         println!("func: {name:?} -> {index:?}");
//                     }
//                 }
//             }
//             SymbolInfo::Data {
//                 flags,
//                 name,
//                 symbol,
//             } => {
//                 if name.contains("_GENERATED") {
//                     let offset = symbol.unwrap().offset;
//                     println!("[{offset}]   Name: {name:?} -> {symbol:?}");
//                 }

//                 // println!()
//             }
//             SymbolInfo::Global { flags, index, name } => {}
//             SymbolInfo::Section { flags, section } => {
//                 println!("Section: {section:?} with flags {flags:?}");
//             }
//             SymbolInfo::Event { flags, index, name } => {}
//             SymbolInfo::Table { flags, index, name } => {}
//         }
//     }

//     // println!("Section: {sect:?} -> {data:?}");
// }
