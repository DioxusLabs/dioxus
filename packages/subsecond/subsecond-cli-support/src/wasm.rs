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
    FunctionId, FunctionKind, ImportKind, Module,
};

/// Prepares the base module before running wasm-bindgen.
///
/// This tries to work around how wasm-bindgen works by intelligently promoting non-wasm-bindgen functions
/// to the export table.
pub fn prepare_base_module(bytes: &[u8]) -> Result<Vec<u8>> {
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
            SymbolInfo::Func { flags, index, name } => Some((name.unwrap(), *index)),
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
        let FunctionKind::Local(local) = &func.kind else {
            continue;
        };

        if !already_exported.contains(name) {
            pre_bindgen.exports.add(&name, func.id());
            already_exported.insert(name.to_string());
        }
    }

    // Add a mutable global called __IFUNC_OFFSET. When we load patches, they use this as their offset to perform relocations.
    // This makes patches extremely flexible and allows them to be loaded at any time and in any order.
    // When the user reloads the page, we can just give them the last patch and the relocation is automatic.
    // pre_bindgen.exports.add(
    //     "__IFUNC_OFFSET",
    //     pre_bindgen.globals.add_local(
    //         walrus::ValType::I32,
    //         true,
    //         false,
    //         walrus::ConstExpr::Value(walrus::ir::Value::I32(0)),
    //     ),
    // );

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
/// We need to move it to match the base module's ifunc table.
pub fn move_func_initiailizers(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut module = walrus::Module::from_buffer(bytes)?;

    let (ifunc_global, _) =
        module.add_import_global("env", "__IFUNC_OFFSET", walrus::ValType::I32, false, false);

    let table = module.tables.iter_mut().next().unwrap();
    table.initial = 1549;
    let segments = table.elem_segments.clone();

    for seg in segments {
        match &mut module.elements.get_mut(seg).kind {
            walrus::ElementKind::Passive => todo!(),
            walrus::ElementKind::Declared => todo!(),
            walrus::ElementKind::Active { table, offset } => {
                tracing::info!("original offset {:?}", offset);
                *offset = walrus::ConstExpr::Global(ifunc_global);
                // match offset {
                //     walrus::ConstExpr::Value(value) => {
                //         *value = walrus::ir::Value::I32(1700 + 1);
                //     }
                //     walrus::ConstExpr::Global(id) => {}
                //     walrus::ConstExpr::RefNull(ref_type) => {}
                //     walrus::ConstExpr::RefFunc(id) => {}
                // }
            }
        }
    }

    let bindgen_funcs = collect_all_wasm_bindgen_funcs(&module);

    // Due to monomorphizations, functions will get merged and multiple names will point to the same function.
    // Walrus loses this information, so we need to manually parse the names table to get the indices
    // and names of these functions.
    let raw_data = parse_bytes_to_data_segment(bytes)?;

    // name -> index
    // we want to export *all* these functions
    let all_funcs = raw_data
        .iter()
        .flat_map(|sym| match sym {
            SymbolInfo::Func { flags, index, name } => Some((name.unwrap(), *index)),
            _ => None,
        })
        .collect::<HashMap<_, _>>();

    let index_to_func = module.funcs.iter().enumerate().collect::<HashMap<_, _>>();

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

    for (name, index) in all_funcs {
        let func = index_to_func.get(&(index as usize)).unwrap();
        let FunctionKind::Local(local) = &func.kind else {
            continue;
        };

        if !already_exported.contains(name) {
            module.exports.add(&name, func.id());
            already_exported.insert(name.to_string());
        }
    }

    Ok(module.emit_wasm())
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
            walrus::ElementItems::Functions(ids) => ids.len(),
            walrus::ElementItems::Expressions(ref_type, const_exprs) => const_exprs.len(),
        })
        // .map(|table| table.elem_segments.len())
        .max()
        .unwrap_or(1)
}

#[test]
fn test_prepare_base_module() {
    prepare_base_module(include_bytes!("../../data/wasm-1/pre-bindgen.wasm"));
}

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
