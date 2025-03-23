//!
//!
//!
//! The process for expanding the base module involves:
//! 1. Creating spare space in the ifunc table for new entries
//! 2. Create an ifunc "mirror" for every real function that could be patched so the side module can call its functions
//! 3. Export necessary items (globals) from the host (and prevent them from getting gc-ed!)
//! 4. Adjusting ifunc indices in the patch module to initialize in the new space from the base module
//! 5. Load the patch module which runs its initializers into the table slots
//! 6. When detouring the function, we need to call the appropriate ifunc index with the correct type
//!
//! When building the base module:
//! 1. Make the ifunc table growable
//! 2. Create a call_indirect shim for every function type
//! 3. Make sure to register all functions and globals with the ifunc table so they don't get gc-ed
//!
//! When building the patch module:
//! 1. Build the jump table with the correct indices (based on func_idx and type_idx)
//! 2. Move the ifunc initializers to the correct location
//!
//! Notes:
//! - When the user reloads the page, the patch will be lost. Either we need to reapply the patch or
//!   compile again (completely).
//! - We could overwrite the ifunc table on new patches or just grow it. Overwriting would make the
//!   patching system stateless, but could lead to corruption issues if old funcs and mixed with new funcs.
//! - We could store the ifunc table length in a global and use a expression table initializer rather than hardcoding
//!   the indices in the patch module. That's a very interesting idea. Will try and see if we can get it working.
//! - Actually *calling* the ifunc is a bit tricky. We need to "import" a function that matches the right
//!   signature and then call it.
//! - If the function was already indirect (say a vtable/trait object) then that entry should already
//!   exist in the ifunc table. Just at a different index (and maybe a different typeidx?)
//! - Calling it will involve generating an extern_c function for that type and calling it. During the
//!   base module assembly process, we need to satisfy these imports.
//! - The patch module will need to call functions from the host and those will either need to be:
//!     1) in the ifunc table already or 2) generated for the patch module
//! - Thus I think we need to generate an indirect call for every function since the patch modules might want to call them (but can't).
//!   We could use imports/exports to solve that too which might actually be easier (direct calls, isolated spaces, no need to worry about ifuncs).
//! - wasm_bindgen might be very difficult to work with. ugh. either it doesn't matter on the patch module or we need to run it on the patch.
//!  in theory we should be running it on the patch but adding new bindgen at runtime is gross / might not even be practical.
//! - do we even need to modify the ifunc table? we can just pump the exported functions out into an object and have the patch module call them.
//! - wait yeah this whole thing is much much easier than I thought. we just need the name of the symbol / function index and we can call it directly.
//!
//! new approach:
//! - post-process the post bindgen module and export its functions

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ops::Range,
    path::PathBuf,
};
use wasmparser::{
    BinaryReader, Linking, LinkingSectionReader, Payload, RelocSectionReader, RelocationEntry,
    SymbolInfo,
};

use anyhow::{Context, Result};
use tokio::process::Command;
use walrus::{
    ir::{dfs_in_order, Visitor},
    FunctionId, FunctionKind, Module,
};

#[test]
fn test_ensure_matching() {
    ensure_matching().unwrap();
}

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

pub fn get_ifunc_table_length(bytes: &[u8]) -> usize {
    let module = walrus::Module::from_buffer(bytes).unwrap();
    module
        .tables
        .iter()
        .map(|table| table.elem_segments.iter())
        .flatten()
        .map(|segment| match &module.elements.get(*segment).items {
            walrus::ElementItems::Functions(ids) => ids.len(),
            walrus::ElementItems::Expressions(ref_type, const_exprs) => const_exprs.len(),
        })
        // .map(|table| table.elem_segments.len())
        .max()
        .unwrap_or(1)
}

/// Prepares the base module before running wasm-bindgen.
///
/// This tries to work around how wasm-bindgen works by intelligently promoting non-wasm-bindgen functions
/// to the export table.
pub fn prepare_base_module(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut pre_bindgen = walrus::Module::from_buffer(bytes)?;

    let bindgen_funcs = collect_all_wasm_bindgen_funcs(&pre_bindgen);

    let raw_data = parse_bytes_to_data_segment(bytes)?;

    for func in bindgen_funcs.iter() {
        let name = pre_bindgen.funcs.get(*func).name.as_ref().unwrap();
        // tracing::warn!("Wasm-bindgen function: {}", name);
    }

    let funcs_to_export = pre_bindgen
        .funcs
        .iter()
        .filter(|func| !bindgen_funcs.contains(&func.id()))
        .filter(|func| matches!(func.kind, FunctionKind::Local(_)))
        .map(|func| func.id())
        .collect::<HashSet<_>>();

    let mut already_exported = pre_bindgen
        .exports
        .iter()
        .map(|exp| exp.name.clone())
        .collect::<HashSet<_>>();

    // tracing::info!("Already exported: {:#?}", already_exported);

    for import in pre_bindgen.imports.iter() {
        tracing::error!("Import: {}", import.name);
        // let name = import.name
        // if name.contains("_ZN59_") {
        //     // if name.contains("dyn$u20$core..any..Any$u20$as$u20$core..fmt..Debug$GT$3fmt") {
        //     tracing::error!("found?: {}", name);
        // }
    }
    for func in pre_bindgen.funcs.iter() {
        let name = func.name.as_ref().unwrap();
        tracing::error!("Func [{}]: {}", func.id().index(), name);
        // if name.contains("_ZN59_") {
        // [2m2025-03-23T09:22:07.067150Z[0m [32m INFO[0m [2msubsecond_cli_support::wasm[0m[2m:[0m Func [28878]: _ZN59_$LT$dyn$u20$core..any..Any$u20$as$u20$core..fmt..Debug$GT$3fmt17haa1f6a0961c11078E
        // if name.contains("dyn$u20$core..any..Any$u20$as$u20$core..fmt..Debug$GT$3fmt") {
        //     tracing::error!("found?: {}", name);
        // }
    }

    for func in funcs_to_export {
        let func = pre_bindgen.funcs.get(func);
        let name = func.name.as_ref().unwrap();
        // if name.contains("a1f6a0961c1107") {
        //     tracing::error!("Skipping function: {}", name);
        // }

        if !already_exported.contains(name) {
            // tracing::info!("Exporting function: {}", name);
            pre_bindgen.exports.add(&name, func.id());
            already_exported.insert(name.clone());
        }
    }

    Ok(pre_bindgen.emit_wasm())
}

/// Collect all the wasm-bindgen functions in the module. We are going to make *everything* exported
/// but we don't want to make *these* exported.
fn collect_all_wasm_bindgen_funcs(module: &Module) -> HashSet<FunctionId> {
    const PREFIX: &str = "__wbindgen_describe_";

    let mut acc = AccAllDescribes::default();
    for func in module.funcs.iter() {
        let name = func.name.as_ref().unwrap();

        // Only deal with the __wbindgen_describe_ functions
        if !(name.starts_with(PREFIX)
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

    /// The __wbindgen_describe_ functions also reference funcs like _ZN86_$LT$dioxus_web..document..JSOwner$u20$as$u20$wasm_bindgen..describe..WasmDescribe$GT$8describe17ha9b39368d518c1f9E
    /// These can be found by walking the instructions.
    #[derive(Default)]
    struct AccAllDescribes {
        funcs: HashSet<FunctionId>,
    }
    impl<'a> Visitor<'a> for AccAllDescribes {
        fn visit_function_id(&mut self, function: &walrus::FunctionId) {
            self.funcs.insert(*function);
        }
    }

    tracing::info!("Found {} wasm-bindgen functions", acc.funcs.len());

    acc.funcs
}

#[test]
fn test_prepare_patch_module() {
    // --import-undefined
    // --import-memory
    // --unresolved-symbols=ignore-all
    // --allow-undefined
    //   --relocatable           Create relocatable object file
    //   --table-base=<value>    Table offset at which to place address taken functions (Defaults to 1)
    //
    // seems like we can just use these - import undefined and adjusted table base - to do this all within the linker
    // just requires massaging the base module a bit
    // do we need to run wasm-bindgen on this??
    prepare_patch_module(include_bytes!("../../data/wasm-1/patch.wasm"));
}

fn prepare_patch_module(bytes: &[u8]) -> Result<()> {
    let mut patch = walrus::Module::from_buffer(bytes)?;

    for func in patch.funcs.iter() {
        let name = func.name.as_ref().unwrap();
        // if name.contains("describe") {
        println!(
            "Function [{}]: {}",
            matches!(func.kind, FunctionKind::Local(_)),
            name
        );
        // }
        // println!("Function: {}", name);
    }

    Ok(())
}

async fn link_incrementals() {
    let incrs = include_str!("./wasm-incrs.txt")
        .lines()
        .filter(|line| line.ends_with(".rcgu.o"))
        .collect::<Vec<_>>();

    println!("{:?}", incrs);

    let res = Command::new(wasm_ld().await)
        .args(incrs)
        .arg("--growable-table")
        .arg("--export")
        .arg("main")
        .arg("--export=__heap_base")
        .arg("--export=__data_end")
        .arg("-z")
        .arg("stack-size=1048576")
        .arg("--stack-first")
        .arg("--allow-undefined")
        .arg("--no-demangle")
        .arg("--no-entry")
        // .arg("--no-gc-sections")
        .arg("-o")
        .arg(wasm_data_folder().join("patch.wasm"))
        .output()
        .await
        .unwrap();

    let err = String::from_utf8(res.stderr).unwrap();
    let out = String::from_utf8(res.stdout).unwrap();
    println!("{}", err);
}

async fn wasm_ld() -> PathBuf {
    sysroot()
        .await
        .join("lib/rustlib/aarch64-apple-darwin/bin/gcc-ld/wasm-ld")
}

async fn sysroot() -> PathBuf {
    let res = Command::new("rustc")
        .arg("--print")
        .arg("sysroot")
        .output()
        .await
        .unwrap();

    let path = String::from_utf8(res.stdout).unwrap();
    PathBuf::from(path.trim())
}

fn wasm_data_folder() -> PathBuf {
    subsecond_folder().join("data").join("wasm")
}

fn static_folder() -> PathBuf {
    subsecond_folder().join("subsecond-harness").join("static")
}

/// Folder representing dioxus/packages/subsecond
fn subsecond_folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../")
        .canonicalize()
        .unwrap()
}

/// The incoming module is expecting to initialize its functions at address 1.
///
/// We need to move it to match the base module's ifunc table.
pub fn move_func_initiailizers(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut module = walrus::Module::from_buffer(bytes)?;

    let table = module.tables.iter_mut().next().unwrap();
    table.initial = 1549;
    let segments = table.elem_segments.clone();

    for seg in segments {
        match &mut module.elements.get_mut(seg).kind {
            walrus::ElementKind::Passive => todo!(),
            walrus::ElementKind::Declared => todo!(),
            walrus::ElementKind::Active { table, offset } => {
                tracing::info!("original offset {:?}", offset);
                match offset {
                    walrus::ConstExpr::Value(value) => {
                        *value = walrus::ir::Value::I32(1549 + 1);
                    }
                    walrus::ConstExpr::Global(id) => {}
                    walrus::ConstExpr::RefNull(ref_type) => {}
                    walrus::ConstExpr::RefFunc(id) => {}
                }
            }
        }
    }

    Ok(module.emit_wasm())
}

struct RawDataSection<'a> {
    data_range: Range<usize>,
    symbols: Vec<SymbolInfo<'a>>,
    data_symbols: BTreeMap<usize, DataSymbol>,
}

#[derive(Debug)]
struct DataSymbol {
    index: usize,
    range: Range<usize>,
    segment_offset: usize,
    symbol_size: usize,
    which_data_segment: usize,
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
                segments = section.into_iter().collect::<Result<Vec<_>, _>>()?
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
            _ => {}
        }
    }

    // Accumulate the data symbols into a btreemap for later use
    let mut data_symbols = BTreeMap::new();
    for (index, symbol) in symbols.iter().enumerate() {
        match symbol {
            SymbolInfo::Func { flags, index, name } => {
                if let Some(name) = name {
                    tracing::info!("Func [{index}]: {}", name);
                }
            }
            SymbolInfo::Data {
                flags,
                name,
                symbol,
            } => {
                tracing::info!("Data: {}", name);
            }
            SymbolInfo::Global { flags, index, name } => {}
            SymbolInfo::Section { flags, section } => {}
            SymbolInfo::Event { flags, index, name } => {}
            SymbolInfo::Table { flags, index, name } => {}
        }

        let SymbolInfo::Data {
            symbol: Some(symbol),
            ..
        } = symbol
        else {
            continue;
        };

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
                index,
                range,
                segment_offset: symbol.offset as usize,
                symbol_size: symbol.size as usize,
                which_data_segment: symbol.index as usize,
            },
        );
    }

    Ok(RawDataSection {
        data_range,
        symbols,
        data_symbols,
    })
}

#[tokio::test]
async fn test_link_incrementals() {
    link_incrementals().await;
}

#[test]
fn test_prepare_base_module() {
    prepare_base_module(include_bytes!("../../data/wasm-1/pre-bindgen.wasm"));
}
