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
    ir::{self},
    ConstExpr, ElementItems, ElementKind, ExportItem, FunctionBuilder, FunctionId, FunctionKind,
    ImportKind, LocalFunction, Module, ModuleConfig, TableId,
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
        return Ok(create_wasm_jump_table(original, patch).unwrap());
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

    let ParsedModule {
        module: old,
        ids: old_ids,
        symbols: old_raw_data,
        ..
    } = parse_module_with_ids(&old_bytes)?;
    let ParsedModule {
        module: mut new,
        ids: new_ids,
        symbols: new_raw_data,
        ..
    } = parse_module_with_ids(&new_bytes)?;

    let name_to_ifunc_old = collect_func_ifuncs(&old, &old_raw_data, &old_ids);
    let name_to_ifunc_old = fill_ifuncs_from_old(name_to_ifunc_old, &old, &old_raw_data);
    if old_raw_data.symbols.is_empty() {
        tracing::warn!("No debug symbols in the WASM output. Make sure to set `opt-level = 0` for hotpatching to work properly.");
        return Err(PatchError::MissingSymbols { symbols: vec![] });
    }

    let exports_to_funcids = old
        .exports
        .iter()
        .filter_map(|e| match e.item {
            ExportItem::Function(id) => Some((e.name.as_str(), id)),
            _ => None,
        })
        .collect::<HashMap<_, _>>();

    let mut mems = vec![];
    let mut funcs = vec![];
    let mut wbg_funcs = vec![];
    // let mut make_env_import = vec![];
    // let mut to_ifuncs = vec![];

    // Collect all the GOT entries from the new module.
    'import_iter: for t in new.imports.iter() {
        match t.module.as_str() {
            "GOT.func" => {
                match name_to_ifunc_old.get(t.name.as_str()).cloned() {
                    Some(entry) => funcs.push((t.id(), entry)),
                    _ => {
                        // match exists {
                        //     Ok(export) => {
                        // tracing::info!("Found GOT.func entry as export: {t:?} -> {export:?}");
                        let sym_index = old_raw_data.code_symbol_map.get(t.name.as_str());
                        for s in old_raw_data.symbols.iter() {
                            if let SymbolInfo::Func {
                                index,
                                name: Some(name),
                                ..
                            } = s
                            {
                                if let Some(sym_index) = sym_index {
                                    if *index == *sym_index as u32 {
                                        if let Some(ifunc) = name_to_ifunc_old.get(name) {
                                            tracing::info!("Found GOT.func entry as symbol: {t:?} -> {ifunc:?}");
                                            funcs.push((t.id(), *ifunc));
                                            continue 'import_iter;
                                        }
                                    }
                                }
                            }
                        }

                        let exists = old.exports.get_func(t.name.as_str());
                        return Err(PatchError::InvalidModule(format!("Expected to find GOT.func entry in ifunc table but it was missing: {} -> {exists:?}\nDid all symbols make it into the static lib?", t.name.as_str())));
                    }
                };
            }
            "GOT.mem" => mems.push(t.id()),
            "env" => {
                // tracing::debug!("Found env import: {t:?}");
                // to_ifuncs.push(t.id());
            }
            "__wbindgen_placeholder__" => {
                wbg_funcs.push(t.id());
            }
            m => {
                tracing::info!("Unknown import module: {m} -> {}", t.name);
                // let Some(entry) = name_to_ifunc_old.get(t.name.as_str()).cloned() else {
                //     let exists = old.exports.get_func(t.name.as_str());
                //     return Err(PatchError::InvalidModule(format!("Expected to find <{m}> in ifunc table but it was missing: {} -> {exists:?}\nDid all symbols make it into the static lib?", t.name.as_str())));
                // };
                // funcs.push((t.id(), entry));
            }
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

    // Conver the env func imports into ifuncs
    let ifunc_table_initializer = new
        .elements
        .iter()
        .find_map(|e| match e.kind {
            ElementKind::Active { table, .. } => Some(table),
            _ => None,
        })
        .context("Missing ifunc table")?;

    for t in wbg_funcs {
        let import = new.imports.get_mut(t);
        let matching_export = old
            .exports
            .iter()
            .find(|e| e.name.contains(import.name.as_str()));
        let exists_as_ifunc_maybe = name_to_ifunc_old.keys().find(|k| k.contains(&import.name));

        tracing::debug!(
            "Converting placeholder to synthesized: {} ({matching_export:?}) or {exists_as_ifunc_maybe:?}",
            import.name
        );

        if let Some(matchin) = matching_export {
            if let ExportItem::Function(id) = matchin.item {
                let ty = old.funcs.get(id).ty();
                let ty = old.types.get(ty);
                tracing::trace!("type of synth: {ty:?}");
            }

            if let ImportKind::Function(id) = import.kind {
                let ty = new.funcs.get(id).ty();
                let ty = new.types.get(ty);
                tracing::trace!("type of inc: {ty:?}");
            }

            import.module = "env".into();
            import.name = matchin.name.clone();
        }
    }

    // Update the wasm module on the filesystem to use the newly lifted version
    let lib = patch.to_path_buf();
    std::fs::write(&lib, new.emit_wasm())?;

    // And now assemble the jump table by mapping the old ifunc table to the new one, by name
    //
    // The ifunc_count will be passed to the dynamic loader so it can allocate the right amount of space
    // in the indirect function table when loading the patch.
    let name_to_ifunc_new = collect_func_ifuncs(&new, &new_raw_data, &new_ids);
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

fn convert_import_to_ifunc_call(
    new: &mut Module,
    ifunc_table_initializer: TableId,
    func_id: FunctionId,
    table_idx: i32,
) {
    let func = new.funcs.get_mut(func_id);
    let ty_id = func.ty();

    // Convert the import function to a local function that calls the indirect function from the table
    let ty = new.types.get(ty_id);

    let params = ty.params().to_vec();
    let results = ty.results().to_vec();
    let args: Vec<_> = params.iter().map(|ty| new.locals.add(*ty)).collect();

    // New function that calls the indirect function
    let mut builder = FunctionBuilder::new(&mut new.types, &params, &results);
    let mut body = builder.name("stub".into()).func_body();

    // Push the params onto the stack
    for arg in args.iter() {
        body.local_get(*arg);
    }

    // And then the address of the indirect function
    body.instr(ir::Instr::Const(ir::Const {
        value: ir::Value::I32(table_idx),
    }));

    // And call it
    body.instr(ir::Instr::CallIndirect(ir::CallIndirect {
        ty: ty_id,
        table: ifunc_table_initializer,
    }));

    new.funcs.get_mut(func_id).kind = FunctionKind::Local(builder.local_func(args));
}

fn fill_ifuncs_from_old<'a>(
    func_to_offset: HashMap<&'a str, i32>,
    m: &'a Module,
    raw: &'a RawDataSection<'a>,
) -> HashMap<&'a str, i32> {
    // These are the "real" bindings for functions in the module
    // Basically a map between a function's index and its real name
    let func_to_index = m
        .funcs
        .iter()
        .filter_map(|f| {
            let name = f.name.as_deref()?;
            Some((*raw.code_symbol_map.get(name)?, name))
        })
        .collect::<HashMap<usize, &str>>();

    // Find the corresponding function that shares the same index, but in the ifunc table
    raw.code_symbol_map
        .iter()
        .filter_map(|(name, idx)| {
            let new_modules_unified_function = func_to_index.get(idx)?;
            let offset = func_to_offset.get(new_modules_unified_function)?;
            Some((*name, *offset as i32))
        })
        .collect()
}

fn collect_func_ifuncs<'a>(
    m: &'a Module,
    raw: &'a RawDataSection<'a>,
    ids_to_fns: &[FunctionId],
) -> HashMap<&'a str, i32> {
    // Collect all the functions in the module that are ifuncs
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
                    let name = m.funcs.get(*id).name.as_deref().unwrap();
                    func_to_offset.insert(name, offset + idx as i32);
                }
            }
            ElementItems::Expressions(_ref_type, _const_exprs) => {}
        }
    }

    func_to_offset
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

    let already_exported = module
        .exports
        .iter()
        .filter(|f| matches!(f.item, ExportItem::Function(_)))
        .map(|exp| exp.name.clone())
        .collect::<HashSet<_>>();

    let imported_funcs = module
        .imports
        .iter()
        .filter_map(|i| match i.kind {
            ImportKind::Function(id) => Some((id, i.id())),
            _ => None,
        })
        .collect::<HashMap<_, _>>();

    // Wasm-bindgen will synthesize imports to satisfy its external calls. This facilitiates things
    // like inline-js, snippets, and literally the `#[wasm_bindgen]` macro. All calls to JS are
    // just `extern "C"` blocks!
    //
    // However, wasm-bindgen will run a GC pass on the module, removing any unused imports.
    let mut make_indirect = vec![];
    for (imported_func, importid) in imported_funcs {
        let import = module.imports.get(importid);
        let name_is_wbg =
            import.name.starts_with("__wbindgen") || import.name.starts_with("__wbg_");

        if name_is_wbg && !name_is_bindgen_symbol(import.name.as_str()) {
            let func = module.funcs.get(imported_func);

            let ty = module.types.get(func.ty());
            let params = ty.params().to_vec();
            let results = ty.results().to_vec();

            let mut builder = FunctionBuilder::new(&mut module.types, &params, &results);
            let mut body = builder
                .name(format!("__saved_wbg_{}", import.name))
                .func_body();

            let locals = params
                .iter()
                .map(|ty| module.locals.add(*ty))
                .collect::<Vec<_>>();

            for l in locals.iter() {
                body.local_get(*l);
            }

            body.call(imported_func);

            let new_func_id = module.funcs.add_local(builder.local_func(locals));

            module
                .exports
                .add(&format!("__saved_wbg_{}", import.name), new_func_id);

            make_indirect.push(new_func_id);
        }
    }

    for (name, index) in symbols.code_symbol_map.iter() {
        if name_is_bindgen_symbol(name) {
            continue;
        }

        let func = module.funcs.get(ids[*index]);

        // We want to preserve the intrinsics from getting gc-ed out.
        //
        // These will create corresponding shim functions in the main module, that the patches will
        // then call. Wasm-bindgen doesn't actually check if anyone uses the `__wbindgen` exports and
        // forcefully deletes them literally by checking for symbols that start with `__wbindgen`. We
        // preserve these symbols by naming them `__saved_wbg_<name>` and then exporting them.
        //
        // When wasm-bindgen runs, it will wrap these intrinsics with an `externref shim`, but we
        // want to preserve the actual underlying function so side modules can call them directly.
        //
        // https://github.com/rustwasm/wasm-bindgen/blob/c35cc9369d5e0dc418986f7811a0dd702fb33ef9/crates/cli-support/src/wit/mod.rs#L1505
        if name.starts_with("__wbindgen") {
            module
                .exports
                .add(&format!("__saved_wbg_{name}"), func.id());
        }

        // This is basically `--export-all` but designed to work around wasm-bindgen not properly gc-ing
        // imports like __wbindgen_placeholder__ and __wbindgen_externref__
        //
        // We only export local functions, and then make sure they can be accessible indirectly.
        // If we weren't dealing with PIC code, then we could just create local ifuncs in the patch that
        // call the original function directly. Unfortunately, this would require adding a new relocation
        // to corresponding GOT.func entry, which we don't want to deal with.
        //
        // By exposing all functions both as exports and ifuncs, we can both call them directly and
        // indirectly.
        if let FunctionKind::Local(_) = &func.kind {
            if !already_exported.contains(*name) {
                module.exports.add(name, func.id());
            }

            if !ifuncs.contains(&func.id()) {
                make_indirect.push(func.id());
            }
        }
    }

    // Now we need to make sure to add the new ifuncs to the ifunc segment initializer.
    // We just assume the last segment is the safest one we can add to which is common practice.
    let segment = module
        .elements
        .iter_mut()
        .last()
        .context("Missing ifunc table")?;
    let make_indirect_count = make_indirect.len() as u64;
    let ElementItems::Functions(segment_ids) = &mut segment.items else {
        return Err(PatchError::InvalidModule(
            "Expected ifunc table to be a function table".into(),
        ));
    };

    for func in make_indirect {
        segment_ids.push(func);
    }

    if let ElementKind::Active { table, .. } = segment.kind {
        let table = module.tables.get_mut(table);
        table.initial += make_indirect_count;
        if let Some(max) = table.maximum {
            table.maximum = Some(max + make_indirect_count);
        }
    }

    Ok(module.emit_wasm())
}

/// Check if the name is a wasm-bindgen symbol
///
/// todo(jon): I believe we can just look at all the functions the wasm_bindgen describe export references.
/// this is kinda hacky on slow.
fn name_is_bindgen_symbol(name: &str) -> bool {
    // https://github.com/rustwasm/wasm-bindgen/blob/c35cc9369d5e0dc418986f7811a0dd702fb33ef9/crates/cli-support/src/wit/mod.rs#L1165
    name.contains("__wbindgen_describe")
        || name.contains("__wbindgen_externref")
        || name.contains("wasm_bindgen8describe6inform")
        || name.contains("wasm_bindgen..describe..WasmDescribe")
        || name.contains("wasm_bindgen..closure..WasmClosure$GT$8describe")
        || name.contains("wasm_bindgen7closure16Closure$LT$T$GT$4wrap8describe")
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

// if *name == "_ZN4core3fmt3num3imp54_$LT$impl$u20$core..fmt..Display$u20$for$u20$usize$GT$3fmt17h6d9dbc09b6dc47c8E" {
// // if *name == "_ZN42_$LT$$RF$T$u20$as$u20$core..fmt..Debug$GT$3fmt17h8dcb6eecee078060E" {
//     // if name.contains("3fmt17h8dcb6eecee07806") {
//     tracing::debug!("Found a special function: {name}");
//     tracing::debug!("Kind: {:?}", func.kind);
//     tracing::debug!("already exported? {:?}", already_exported.contains(*name));
//     tracing::debug!("ifuncs contains it?: {:?}", ifuncs.get(&func.id()));
// }

// if name_is_bindgen_symbol(name) {
//     /// The __wbindgen_describe_ functions also reference funcs like:
//     /// _ZN86_$LT$dioxus_web..document..JSOwner$u20$as$u20$wasm_bindgen..describe..WasmDescribe$GT$8describe17ha9b39368d518c1f9E
//     ///
//     /// These can be found by walking the instructions, so we build a Visitor
//     /// ... todo: we might not need to do this since it seems that it's not reliable enough
//     #[derive(Default)]
//     struct AccAllDescribes {
//         funcs: HashSet<FunctionId>,
//     }

//     impl<'a> Visitor<'a> for AccAllDescribes {
//         fn visit_function_id(&mut self, function: &walrus::FunctionId) {
//             self.funcs.insert(*function);
//         }
//     }

//     // let mut acc = AccAllDescribes::default();
//     // for func in module.funcs.iter() {
//     //     let Some(name) = func.name.as_ref() else {
//     //         continue;
//     //     };

//     //     // Only deal with the __wbindgen_describe_ functions
//     //     if !name_is_bindgen_symbol(name) {
//     //         continue;
//     //     }

//     //     // They call other functions, so we need to find those too and make sure not to mark them as exported
//     //     if let FunctionKind::Local(func) = &module.funcs.get(func.id()).kind {
//     //         walrus::ir::dfs_in_order(&mut acc, &func, func.entry_block());
//     //     }

//     //     acc.funcs.insert(func.id());
//     // }

//     // if let FunctionKind::Local(local) = &func.kind {
//     //     let mut accer = AccAllDescribes::default();
//     //     walrus::ir::dfs_in_order(&mut accer, &local, local.entry_block());
//     //     for func in accer.funcs {
//     //         if let Some(new_name) = module.funcs.get(func).name.as_deref() {
//     //             if !name_is_bindgen_symbol(new_name) {
//     //                 tracing::warn!("references real {name} -> {new_name}");
//     //                 //     // make_indirect.push(func);
//     //             }
//     //         }
//     //     }
//     // }

//     continue;
// }

// // if name.contains("__wbindgen") || name.contains("wbg") {
// //     tracing::debug!("sus name: {name}");
// // }

// https://github.com/rustwasm/wasm-bindgen/blob/c35cc9369d5e0dc418986f7811a0dd702fb33ef9/src/lib.rs#L1061-L1160
// externs! {
//     #[link(wasm_import_module = "__wbindgen_placeholder__")]
//     extern "C" {
//         fn __wbindgen_object_clone_ref(idx: u32) -> u32;
//         fn __wbindgen_object_drop_ref(idx: u32) -> ();

//         fn __wbindgen_string_new(ptr: *const u8, len: usize) -> u32;
//         fn __wbindgen_number_new(f: f64) -> u32;
//         fn __wbindgen_bigint_from_str(ptr: *const u8, len: usize) -> u32;
//         fn __wbindgen_bigint_from_i64(n: i64) -> u32;
//         fn __wbindgen_bigint_from_u64(n: u64) -> u32;
//         fn __wbindgen_bigint_from_i128(hi: i64, lo: u64) -> u32;
//         fn __wbindgen_bigint_from_u128(hi: u64, lo: u64) -> u32;
//         fn __wbindgen_symbol_named_new(ptr: *const u8, len: usize) -> u32;
//         fn __wbindgen_symbol_anonymous_new() -> u32;

//         fn __wbindgen_externref_heap_live_count() -> u32;

//         fn __wbindgen_is_null(idx: u32) -> u32;
//         fn __wbindgen_is_undefined(idx: u32) -> u32;
//         fn __wbindgen_is_symbol(idx: u32) -> u32;
//         fn __wbindgen_is_object(idx: u32) -> u32;
//         fn __wbindgen_is_array(idx: u32) -> u32;
//         fn __wbindgen_is_function(idx: u32) -> u32;
//         fn __wbindgen_is_string(idx: u32) -> u32;
//         fn __wbindgen_is_bigint(idx: u32) -> u32;
//         fn __wbindgen_typeof(idx: u32) -> u32;

//         fn __wbindgen_in(prop: u32, obj: u32) -> u32;

//         fn __wbindgen_is_falsy(idx: u32) -> u32;
//         fn __wbindgen_as_number(idx: u32) -> f64;
//         fn __wbindgen_try_into_number(idx: u32) -> u32;
//         fn __wbindgen_neg(idx: u32) -> u32;
//         fn __wbindgen_bit_and(a: u32, b: u32) -> u32;
//         fn __wbindgen_bit_or(a: u32, b: u32) -> u32;
//         fn __wbindgen_bit_xor(a: u32, b: u32) -> u32;
//         fn __wbindgen_bit_not(idx: u32) -> u32;
//         fn __wbindgen_shl(a: u32, b: u32) -> u32;
//         fn __wbindgen_shr(a: u32, b: u32) -> u32;
//         fn __wbindgen_unsigned_shr(a: u32, b: u32) -> u32;
//         fn __wbindgen_add(a: u32, b: u32) -> u32;
//         fn __wbindgen_sub(a: u32, b: u32) -> u32;
//         fn __wbindgen_div(a: u32, b: u32) -> u32;
//         fn __wbindgen_checked_div(a: u32, b: u32) -> u32;
//         fn __wbindgen_mul(a: u32, b: u32) -> u32;
//         fn __wbindgen_rem(a: u32, b: u32) -> u32;
//         fn __wbindgen_pow(a: u32, b: u32) -> u32;
//         fn __wbindgen_lt(a: u32, b: u32) -> u32;
//         fn __wbindgen_le(a: u32, b: u32) -> u32;
//         fn __wbindgen_ge(a: u32, b: u32) -> u32;
//         fn __wbindgen_gt(a: u32, b: u32) -> u32;

//         fn __wbindgen_number_get(idx: u32) -> WasmRet<Option<f64>>;
//         fn __wbindgen_boolean_get(idx: u32) -> u32;
//         fn __wbindgen_string_get(idx: u32) -> WasmSlice;
//         fn __wbindgen_bigint_get_as_i64(idx: u32) -> WasmRet<Option<i64>>;

//         fn __wbindgen_debug_string(ret: *mut [usize; 2], idx: u32) -> ();

//         fn __wbindgen_throw(a: *const u8, b: usize) -> !;
//         fn __wbindgen_rethrow(a: u32) -> !;
//         fn __wbindgen_error_new(a: *const u8, b: usize) -> u32;

//         fn __wbindgen_cb_drop(idx: u32) -> u32;

//         fn __wbindgen_describe(v: u32) -> ();
//         fn __wbindgen_describe_closure(a: u32, b: u32, c: u32) -> u32;

//         fn __wbindgen_json_parse(ptr: *const u8, len: usize) -> u32;
//         fn __wbindgen_json_serialize(idx: u32) -> WasmSlice;
//         fn __wbindgen_jsval_eq(a: u32, b: u32) -> u32;
//         fn __wbindgen_jsval_loose_eq(a: u32, b: u32) -> u32;

//         fn __wbindgen_copy_to_typed_array(ptr: *const u8, len: usize, idx: u32) -> ();

//         fn __wbindgen_uint8_array_new(ptr: *mut u8, len: usize) -> u32;
//         fn __wbindgen_uint8_clamped_array_new(ptr: *mut u8, len: usize) -> u32;
//         fn __wbindgen_uint16_array_new(ptr: *mut u16, len: usize) -> u32;
//         fn __wbindgen_uint32_array_new(ptr: *mut u32, len: usize) -> u32;
//         fn __wbindgen_biguint64_array_new(ptr: *mut u64, len: usize) -> u32;
//         fn __wbindgen_int8_array_new(ptr: *mut i8, len: usize) -> u32;
//         fn __wbindgen_int16_array_new(ptr: *mut i16, len: usize) -> u32;
//         fn __wbindgen_int32_array_new(ptr: *mut i32, len: usize) -> u32;
//         fn __wbindgen_bigint64_array_new(ptr: *mut i64, len: usize) -> u32;
//         fn __wbindgen_float32_array_new(ptr: *mut f32, len: usize) -> u32;
//         fn __wbindgen_float64_array_new(ptr: *mut f64, len: usize) -> u32;

//         fn __wbindgen_array_new() -> u32;
//         fn __wbindgen_array_push(array: u32, value: u32) -> ();

//         fn __wbindgen_not(idx: u32) -> u32;

//         fn __wbindgen_exports() -> u32;
//         fn __wbindgen_memory() -> u32;
//         fn __wbindgen_module() -> u32;
//         fn __wbindgen_function_table() -> u32;
//     }
// }

#[test]
fn is_in_ifuncs() {
    let path = "/Users/jonathankelley/Development/docsite/target/dx/dioxus_docs_site/debug/web/public/wasm/dioxus_docs_site_bg.wasm";
    let bytes = fs::read(path).unwrap();
    let module = walrus::Module::from_buffer(&bytes).unwrap();
    let symbols = parse_bytes_to_data_segment(&bytes).unwrap();
    let ifunc_map = collect_func_ifuncs(&module, &symbols, &[]);
    let target = "_ZN4core3fmt3num3imp54_$LT$impl$u20$core..fmt..Display$u20$for$u20$usize$GT$3fmt17h6d9dbc09b6dc47c8E";
    let res = ifunc_map.get(target);
    dbg!(res);

    let res = module
        .funcs
        .iter()
        .find(|f| f.name.as_deref() == Some(target));
    dbg!(res);

    let res = symbols.code_symbol_map.get(target);
    dbg!(res);

    // let symbol = &symbols.symbols[*res.unwrap()];
    // dbg!(symbol);
    let symbol = symbols
        .symbols
        .iter()
        .find(|s| match s {
            SymbolInfo::Func { name, .. } => name.as_deref() == Some(target),
            _ => false,
        })
        .unwrap();
    dbg!(symbol);

    for s in symbols.symbols.iter() {
        match s {
            SymbolInfo::Func { name, index, .. } => {
                if *index == 95157 {
                    println!("Found a special function: {name:?}");
                }
            }
            _ => {}
        }
    }
}
