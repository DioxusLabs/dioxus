use anyhow::Context;
use itertools::Itertools;
use object::{
    macho::{self},
    read::File,
    write::{MachOBuildVersion, StandardSection, Symbol, SymbolSection},
    Endianness, Object, ObjectSymbol, SymbolKind, SymbolScope,
};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ops::{Deref, Range},
    path::Path,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use subsecond_types::*;
use target_lexicon::{OperatingSystem, Triple};
use thiserror::Error;
use walrus::{
    ConstExpr, DataKind, ElementItems, ElementKind, FunctionBuilder, FunctionId, FunctionKind,
    ImportKind, Module, ModuleConfig, TableId,
};
use wasmparser::{
    BinaryReader, BinaryReaderError, Linking, LinkingSectionReader, Payload, SymbolInfo,
};

type Result<T, E = PatchError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum PatchError {
    #[error("Failed to read file: {0}")]
    ReadFs(#[from] std::io::Error),

    #[error("No debug symbols in the patch output. Check your profile's `opt-level` and debug symbols config.")]
    MissingSymbols,

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

/// A cache for the hotpatching engine that stores the original module's parsed symbol table.
/// For large projects, this can shave up to 50% off the total patching time. Since we compile the base
/// module with every symbol in it, it can be quite large (hundreds of MB), so storing this here lets
/// us avoid re-parsing the module every time we want to patch it.
///
/// On the Dioxus Docsite, it dropped the patch time from 3s to 1.1s (!)
pub struct HotpatchModuleCache {
    pub _path: PathBuf,

    // .... wasm stuff
    pub symbol_ifunc_map: HashMap<String, i32>,
    pub old_wasm: Module,
    pub old_bytes: Vec<u8>,
    pub old_exports: HashSet<String>,
    pub old_imports: HashSet<String>,

    // ... native stuff
    pub old_name_to_addr: HashMap<String, u64>,
}

/// Create a jump table for the given original and patch files.
pub fn create_jump_table(
    original: &Path,
    patch: &Path,
    triple: &Triple,
    cache: &mut Option<HotpatchModuleCache>,
) -> Result<JumpTable> {
    // Symbols are stored differently based on the platform, so we need to handle them differently.
    // - Wasm requires the walrus crate and actually modifies the patch file
    // - windows requires the pdb crate and pdb files
    // - nix requires the object crate
    match triple.operating_system {
        OperatingSystem::Windows => create_windows_jump_table(original, patch, triple, cache),
        _ if triple.architecture == target_lexicon::Architecture::Wasm32 => {
            create_wasm_jump_table(original, patch, cache)
        }
        _ => create_nix_jump_table(original, patch, triple, cache),
    }
}

fn create_windows_jump_table(
    original: &Path,
    patch: &Path,
    triple: &Triple,
    cache: &mut Option<HotpatchModuleCache>,
) -> std::result::Result<JumpTable, PatchError> {
    use pdb::FallibleIterator;

    let old_pdb_file = original.with_extension("pdb");
    let new_pdb_file = patch.with_extension("pdb");

    // let pdb_file  = "/Users/jonathankelley/Desktop/Windows Shared/synced!/dioxus-demo.pdb";
    let old_pdb_file_handle = std::fs::File::open(old_pdb_file).unwrap();
    let mut pdbf = pdb::PDB::open(old_pdb_file_handle).unwrap();
    // for s in pdbf.global_symbols().unwrap().iter().iterator() {
    //     println!("PDB Symbol: {:?}", s);
    // }

    let symbol_table = pdbf.global_symbols().unwrap();
    let address_map = pdbf.address_map().unwrap();

    // let name_to_address = symbol_table
    //     .iter()
    //     .filter_map(|f| match f.parse() {
    //         Ok(pdb::SymbolData::Public(data))) => todo!(),
    //         Err(_) => todo!(),
    //     })
    //     .collect::<HashMap<_, _>>();
    let mut symbols = symbol_table.iter();
    while let Some(symbol) = symbols.next().unwrap() {
        match symbol.parse() {
            Ok(pdb::SymbolData::Public(data)) if data.function => {
                // we found the location of a function!
                let rva = data.offset.to_rva(&address_map).unwrap_or_default();
                println!("{} is {}", rva, data.name);
            }
            _ => {}
        }
    }

    // // let old_name_to_addr = &cache.as_mut().unwrap().old_name_to_addr;
    // let obj2_bytes = std::fs::read(patch)?;
    // let obj2 = File::parse(&obj2_bytes as &[u8])?;
    // let mut map = AddressMap::default();
    // let new_syms = obj2.symbol_map();

    // let new_name_to_addr = new_syms
    //     .symbols()
    //     .par_iter()
    //     .map(|s| (s.name(), s.address()))
    //     .collect::<HashMap<_, _>>();

    // for (new_name, new_addr) in new_name_to_addr.iter() {
    //     if let Some(old_addr) = old_name_to_addr.get(*new_name) {
    //         map.insert(*old_addr, *new_addr);
    //     }
    // }

    // let new_base_address = match triple.operating_system {
    //     // The symbol in the symtab is called "_main" but in the dysymtab it is called "main"
    //     OperatingSystem::MacOSX(_) | OperatingSystem::Darwin(_) | OperatingSystem::IOS(_) => {
    //         *new_name_to_addr
    //             .get("_main")
    //             .context("failed to find '_main' symbol in patch")?
    //     }

    //     // No distincation between the two on these platforms
    //     OperatingSystem::Freebsd
    //     | OperatingSystem::Openbsd
    //     | OperatingSystem::Linux
    //     | OperatingSystem::Windows => *new_name_to_addr
    //         .get("main")
    //         .context("failed to find 'main' symbol in patch")?,

    //     // On wasm, it doesn't matter what the address is since the binary is PIC
    //     _ => 0,
    // };

    // let aslr_reference = *old_name_to_addr
    //     .get("_aslr_reference")
    //     .or_else(|| old_name_to_addr.get("aslr_reference"))
    //     .context("failed to find '_aslr_reference' symbol in original module")?;

    // Ok(JumpTable {
    //     lib: patch.to_path_buf(),
    //     map,
    //     new_base_address,
    //     aslr_reference,
    //     ifunc_count: 0,
    // })

    todo!()
}

/// Assemble a jump table for "nix" architectures. This uses the `object` crate to parse both
/// executable's symbol tables and then creates a mapping between the two. Unlike windows, the symbol
/// tables are stored within the binary itself, so we can use the `object` crate to parse them.
///
/// We use the `_aslr_reference` as a reference point in the base program to calculate the aslr slide
/// both at compile time and at runtime.
///
/// This does not work for WASM since the `object` crate does not support emitting the WASM format,
/// and because WASM requires more logic to handle the wasm-bindgen transformations.
fn create_nix_jump_table(
    original: &Path,
    patch: &Path,
    triple: &Triple,
    cache: &mut Option<HotpatchModuleCache>,
) -> Result<JumpTable> {
    if cache.is_none() {
        let obj1_bytes = std::fs::read(original)?;
        let obj1 = File::parse(&obj1_bytes as &[u8])?;
        let old_syms = obj1.symbol_map();
        let old_name_to_addr = old_syms
            .symbols()
            .par_iter()
            .map(|s| (s.name().to_string(), s.address()))
            .collect::<HashMap<_, _>>();
        *cache = Some(HotpatchModuleCache {
            old_name_to_addr,
            _path: original.to_path_buf(),
            old_bytes: obj1_bytes,
            symbol_ifunc_map: Default::default(),
            old_wasm: Module::default(),
            old_exports: Default::default(),
            old_imports: Default::default(),
        });
    }

    let old_name_to_addr = &cache.as_mut().unwrap().old_name_to_addr;
    let obj2_bytes = std::fs::read(patch)?;
    let obj2 = File::parse(&obj2_bytes as &[u8])?;
    let mut map = AddressMap::default();
    let new_syms = obj2.symbol_map();

    let new_name_to_addr = new_syms
        .symbols()
        .par_iter()
        .map(|s| (s.name(), s.address()))
        .collect::<HashMap<_, _>>();

    for (new_name, new_addr) in new_name_to_addr.iter() {
        if let Some(old_addr) = old_name_to_addr.get(*new_name) {
            map.insert(*old_addr, *new_addr);
        }
    }

    let new_base_address = match triple.operating_system {
        // The symbol in the symtab is called "_main" but in the dysymtab it is called "main"
        OperatingSystem::MacOSX(_) | OperatingSystem::Darwin(_) | OperatingSystem::IOS(_) => {
            *new_name_to_addr
                .get("_main")
                .context("failed to find '_main' symbol in patch")?
        }

        // No distincation between the two on these platforms
        OperatingSystem::Freebsd
        | OperatingSystem::Openbsd
        | OperatingSystem::Linux
        | OperatingSystem::Windows => *new_name_to_addr
            .get("main")
            .context("failed to find 'main' symbol in patch")?,

        // On wasm, it doesn't matter what the address is since the binary is PIC
        _ => 0,
    };

    let aslr_reference = *old_name_to_addr
        .get("_aslr_reference")
        .or_else(|| old_name_to_addr.get("aslr_reference"))
        .context("failed to find '_aslr_reference' symbol in original module")?;

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
fn create_wasm_jump_table(
    original: &Path,
    patch: &Path,
    cache: &mut Option<HotpatchModuleCache>,
) -> Result<JumpTable> {
    // This caching step is crucial for performance on large projects. The original module can be
    // quite large (hundreds of MB), so this step drastically speeds it up.
    if cache.is_none() {
        let old_bytes = std::fs::read(original)?;
        let ParsedModule {
            module: old_wasm,
            symbols: old_symbols,
            ..
        } = parse_module_with_ids(&old_bytes)?;

        if old_symbols.symbols.is_empty() {
            return Err(PatchError::MissingSymbols);
        }

        let name_to_ifunc_old = collect_func_ifuncs(&old_wasm);
        let name_to_ifunc_old = fill_ifuncs_from_old(&old_wasm, &old_symbols, name_to_ifunc_old);
        let symbol_ifunc_map = name_to_ifunc_old
            .par_iter()
            .map(|(name, idx)| (name.to_string(), *idx))
            .collect::<HashMap<_, _>>();
        let old_exports = old_wasm
            .exports
            .iter()
            .map(|e| e.name.to_string())
            .collect::<HashSet<_>>();
        let old_imports = old_wasm
            .imports
            .iter()
            .map(|i| i.name.to_string())
            .collect::<HashSet<_>>();
        *cache = Some(HotpatchModuleCache {
            _path: original.to_path_buf(),
            old_bytes,
            symbol_ifunc_map,
            old_exports,
            old_imports,
            old_wasm,
            old_name_to_addr: Default::default(),
        });
    }

    let cache = cache.as_mut().unwrap();
    let name_to_ifunc_old = &cache.symbol_ifunc_map;
    let old = &cache.old_wasm;
    let old_symbols =
        parse_bytes_to_data_segment(&cache.old_bytes).context("Failed to parse data segment")?;
    let new_bytes = std::fs::read(patch).context("Could not read patch file")?;

    let mut new = Module::from_buffer(&new_bytes)?;
    let mut got_mems = vec![];
    let mut got_funcs = vec![];
    let mut wbg_funcs = vec![];
    let mut env_funcs = vec![];

    // Collect all the GOT entries from the new module.
    // The GOT imports come from the wasm-ld implementation of the dynamic linking spec
    //
    // https://github.com/WebAssembly/tool-conventions/blob/main/DynamicLinking.md#imports
    //
    // Normally, the base module would synthesize these as exports, but we're not compiling the base
    // module with `--pie` (nor does wasm-bindgen support it yet), so we need to manually satisfy them.
    //
    // One thing to watch out for here is that GOT.func entries have no visibility to any de-duplication
    // or merging, so we need to take great care in the base module to export *every* symbol even if
    // they point to the same function.
    //
    // The other thing to watch out for here is the __wbindgen_placeholder__ entries. These are meant
    // to be satisfied by wasm-bindgen via manual code generation, but we can't run wasm-bindgen on the
    // patch, so we need to do it ourselves. This involves preventing their elimination in the base module
    // by prefixing them with `__saved_wbg_`. When handling the imports here, we need modify the imported
    // name to match the prefixed export name in the base module.
    for import in new.imports.iter() {
        match import.module.as_str() {
            "GOT.func" => {
                let Some(entry) = name_to_ifunc_old.get(import.name.as_str()).cloned() else {
                    return Err(PatchError::InvalidModule(format!(
                        "Expected to find GOT.func entry in ifunc table: {}",
                        import.name.as_str()
                    )));
                };
                got_funcs.push((import.id(), entry));
            }
            "GOT.mem" => got_mems.push(import.id()),
            "env" => env_funcs.push(import.id()),
            "__wbindgen_placeholder__" => wbg_funcs.push(import.id()),
            m => tracing::trace!("Unknown import: {m}:{}", import.name),
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
    for (import_id, ifunc_index) in got_funcs {
        let import = new.imports.get(import_id);
        let ImportKind::Global(id) = import.kind else {
            return Err(PatchError::InvalidModule(format!(
                "Expected GOT.func import to be a global: {}",
                import.name
            )));
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
    for mem in got_mems {
        let import = new.imports.get(mem);
        let data_symbol_idx = *old_symbols
            .data_symbol_map
            .get(import.name.as_str())
            .with_context(|| {
                format!("Failed to find GOT.mem import by its name: {}", import.name)
            })?;
        let data_symbol = old_symbols
            .data_symbols
            .get(&data_symbol_idx)
            .context("Failed to find data symbol by its index")?;
        let data = old
            .data
            .iter()
            .nth(data_symbol.which_data_segment)
            .context("Missing data segment in the main module")?;

        let offset = match data.kind {
            DataKind::Active {
                offset: ConstExpr::Value(walrus::ir::Value::I32(idx)),
                ..
            } => idx,
            DataKind::Active {
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

    // wasm-bindgen has a limit on the number of exports a module can have, so we need to call the main
    // module's functions indirectly. This is done by dropping the env import and replacing it with a
    // local function that calls the indirect function from the table.
    //
    // https://github.com/emscripten-core/emscripten/issues/22863
    let ifunc_table_initializer = new
        .elements
        .iter()
        .find_map(|e| match e.kind {
            ElementKind::Active { table, .. } => Some(table),
            _ => None,
        })
        .context("Missing ifunc table")?;
    for env_func_import in env_funcs {
        let import = new.imports.get(env_func_import);
        let ImportKind::Function(func_id) = import.kind else {
            continue;
        };

        if cache.old_exports.contains(import.name.as_str())
            || cache.old_imports.contains(import.name.as_str())
        {
            continue;
        }

        if let Some(table_idx) = name_to_ifunc_old.get(import.name.as_str()) {
            let name = import.name.as_str().to_string();
            new.imports.delete(env_func_import);
            convert_import_to_ifunc_call(
                &mut new,
                ifunc_table_initializer,
                func_id,
                *table_idx,
                name,
            );
        }
    }

    // Wire up the preserved intrinsic functions that we saved before running wasm-bindgen to the expected
    // imports from the patch.
    for import_id in wbg_funcs {
        let import = new.imports.get_mut(import_id);
        import.module = "env".into();
        import.name = format!("__saved_wbg_{}", import.name);
    }

    // Wipe away the unnecessary sections
    let customs = new.customs.iter().map(|f| f.0).collect::<Vec<_>>();
    for custom_id in customs {
        let custom = new.customs.get_mut(custom_id).unwrap();
        if custom.name().contains("manganis") || custom.name().contains("__wasm_bindgen") {
            new.customs.delete(custom_id);
        }
    }

    // Clear the start function from the patch - we don't want any code automatically running!
    new.start = None;

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
        if let Some(old_idx) = name_to_ifunc_old.get(*name) {
            map.insert(*old_idx as u64, *idx as u64);
        }
    }

    Ok(JumpTable {
        map,
        lib,
        ifunc_count,
        aslr_reference: 0,
        new_base_address: 0,
    })
}

fn convert_import_to_ifunc_call(
    new: &mut Module,
    ifunc_table_initializer: TableId,
    func_id: FunctionId,
    table_idx: i32,
    name: String,
) {
    use walrus::ir;

    let func = new.funcs.get_mut(func_id);
    let ty_id = func.ty();

    // Convert the import function to a local function that calls the indirect function from the table
    let ty = new.types.get(ty_id);

    let params = ty.params().to_vec();
    let results = ty.results().to_vec();
    let locals: Vec<_> = params.iter().map(|ty| new.locals.add(*ty)).collect();

    // New function that calls the indirect function
    let mut builder = FunctionBuilder::new(&mut new.types, &params, &results);
    let mut body = builder.name(name).func_body();

    // Push the params onto the stack
    for arg in locals.iter() {
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

    new.funcs.get_mut(func_id).kind = FunctionKind::Local(builder.local_func(locals));
}

fn fill_ifuncs_from_old<'a>(
    old: &'a Module,
    raw: &'a RawDataSection<'a>,
    func_to_offset: HashMap<&'a str, i32>,
) -> HashMap<&'a str, i32> {
    // These are the "real" bindings for functions in the module
    // Basically a map between a function's index and its real name
    let func_to_index = old
        .funcs
        .par_iter()
        .filter_map(|f| {
            let name = f.name.as_deref()?;
            Some((*raw.code_symbol_map.get(name)?, name))
        })
        .collect::<HashMap<usize, &str>>();

    // Find the corresponding function that shares the same index, but in the ifunc table
    raw.code_symbol_map
        .par_iter()
        .filter_map(|(name, idx)| {
            let new_modules_unified_function = func_to_index.get(idx)?;
            let offset = func_to_offset.get(new_modules_unified_function)?;
            Some((*name, *offset))
        })
        .collect()
}

fn collect_func_ifuncs(m: &Module) -> HashMap<&str, i32> {
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
                    if let Some(name) = m.funcs.get(*id).name.as_deref() {
                        func_to_offset.insert(name, offset + idx as i32);
                    }
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
///
/// Note - this function is not defined to run on WASM binaries. The `object` crate does not
///
/// todo... we need to wire up the cache
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
        let bytes = std::fs::read(path).with_context(|| format!("failed to read {:?}", path))?;
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
                build_version.minos = (11 << 16) | (0 << 8) | 0; // 11.0.0
                build_version.sdk = (11 << 16) | (0 << 8) | 0; // SDK 11.0.0
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
    let bytes = std::fs::read(source_path)?;
    let source = File::parse(bytes.deref() as &[u8]).context("Failed to parse")?;
    let symbol_table = source
        .symbols()
        .flat_map(|s| Some((s.name().ok()?, s)))
        .collect::<HashMap<_, _>>();

    // Get the offset from the main module and adjust the addresses by the slide
    let aslr_ref_address = symbol_table
        .get("_aslr_reference")
        .or_else(|| symbol_table.get("aslr_reference"))
        .context("Failed to find _aslr_reference symbol")?
        .address();
    let aslr_offset = aslr_reference - aslr_ref_address;

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
                    let rel_addr = abs_addr as i32 - 5; // Relative address (offset from next instruction)
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
    let ifunc_map = collect_func_ifuncs(&module);
    let ifuncs = module
        .funcs
        .par_iter()
        .filter_map(|f| ifunc_map.get(f.name.as_deref()?).map(|_| f.id()))
        .collect::<HashSet<_>>();

    let imported_funcs = module
        .imports
        .iter()
        .filter_map(|i| match i.kind {
            ImportKind::Function(id) => Some((id, i.id())),
            _ => None,
        })
        .collect::<HashMap<_, _>>();

    // Wasm-bindgen will synthesize imports to satisfy its external calls. This facilitates things
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
        // Note that we don't export via the export table, but rather the ifunc table. This is to work
        // around issues on large projects where we hit the maximum number of exports.
        //
        // https://github.com/emscripten-core/emscripten/issues/22863
        if let FunctionKind::Local(_) = &func.kind {
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
///
/// Uses the heuristics from the wasm-bindgen source code itself:
///
/// https://github.com/rustwasm/wasm-bindgen/blob/c35cc9369d5e0dc418986f7811a0dd702fb33ef9/crates/cli-support/src/wit/mod.rs#L1165
fn name_is_bindgen_symbol(name: &str) -> bool {
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

    let symbols = parse_bytes_to_data_segment(bindgened).context("Failed to parse data segment")?;

    Ok(ParsedModule {
        module,
        ids,
        symbols,
    })
}
