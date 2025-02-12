use anyhow::{Context, Result};
use itertools::Itertools;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    hash::Hash,
    ops::Range,
    sync::{Arc, RwLock},
};
use walrus::{
    ir::{self, dfs_in_order, Visitor},
    ConstExpr, DataKind, ElementItems, ElementKind, ExportId, ExportItem, FunctionBuilder,
    FunctionId, FunctionKind, GlobalKind, ImportId, ImportKind, ImportedFunction, Module,
    ModuleConfig, RefType, TableId, TypeId,
};
use wasm_used::Used;
use wasmparser::{
    Linking, LinkingSectionReader, Payload, RelocSectionReader, RelocationEntry, SymbolInfo,
};

pub const MAKE_LOAD_JS: &'static str = include_str!("./__wasm_split.js");

/// A parsed wasm module with additional metadata and functionality for splitting and patching.
///
/// This struct assumes that relocations will be present in incoming wasm binary.
/// Upon construction, all the required metadata will be constructed.
pub struct Splitter<'a> {
    /// The original module we use as a reference
    source_module: Module,

    // The byte sources of the pre and post wasm-bindgen .wasm files
    // We need the original around since wasm-bindgen ruins the relocation locations.
    original: &'a [u8],
    bindgened: &'a [u8],

    // Mapping of indices of source functions
    // This lets us use a much faster approach to emitting split modules simply by maintaing a mapping
    // between the original Module and the new Module. Ideally we could just index the new module
    // with old FunctionIds but the underlying IndexMap actually checks that a key belongs to a particular
    // arena.
    fns_to_ids: HashMap<FunctionId, usize>,
    _ids_to_fns: Vec<FunctionId>,

    split_points: Vec<SplitPoint>,
    chunks: Vec<HashSet<Node>>,
    data_symbols: BTreeMap<usize, DataSymbol>,
    symbols: Vec<SymbolInfo<'a>>,
    main_graph: HashSet<Node>,
    call_graph: HashMap<Node, HashSet<Node>>,
    parent_graph: HashMap<Node, HashSet<Node>>,
    extra_symbols: HashSet<Node>,
}

/// The results of splitting the wasm module with some additional metadata for later use.
pub struct OutputModules {
    /// The main chunk
    pub main: SplitModule,

    /// The modules of the wasm module that were split.
    pub modules: Vec<SplitModule>,

    /// The chunks that might be imported by the main modules
    pub chunks: Vec<SplitModule>,
}

/// A wasm module that was split from the main module.
///
/// All IDs here correspond to *this* module - not the parent main module
pub struct SplitModule {
    pub module_name: String,
    pub hash_id: Option<String>,
    pub component_name: Option<String>,
    pub bytes: Vec<u8>,
    pub relies_on_chunks: HashSet<usize>,
}

impl<'a> Splitter<'a> {
    /// Create a new "splitter" instance using the original wasm and the wasm from the output of wasm-bindgen.
    ///
    /// This will use the relocation data from the original module to create a call graph that we
    /// then use with the post-bindgened module to create the split modules.
    ///
    /// It's important to compile the wasm with --emit-relocs such that the relocations are available
    /// to construct the callgraph.
    pub fn new(original: &'a [u8], bindgened: &'a [u8]) -> Result<Self> {
        let (module, ids, fns_to_ids) = parse_module_with_ids(bindgened)?;

        let split_points = accumulate_split_points(&module);
        let raw_data = parse_bytes_to_data_segment(&bindgened)?;

        let mut module = Self {
            source_module: module,
            original,
            bindgened,
            split_points,
            data_symbols: raw_data.data_symbols,
            symbols: raw_data.symbols,
            _ids_to_fns: ids,
            fns_to_ids,
            main_graph: Default::default(),
            chunks: Default::default(),
            call_graph: Default::default(),
            parent_graph: Default::default(),
            extra_symbols: Default::default(),
        };

        module.build_call_graph()?;
        // module.build_split_chunks();

        Ok(module)
    }

    /// Split the module into multiple modules at the boundaries of split points.
    ///
    /// Note that the binaries might still be "large" at the end of this process. In practice, you
    /// need to push these binaries through wasm-bindgen and wasm-opt to take advantage of the
    /// optimizations and splitting. We perform a few steps like zero-ing out the data segments
    /// that will only be removed by the memory-packing step of wasm-opt.
    ///
    /// This returns the list of chunks, an import map, and some javascript to link everything together.
    pub fn emit(self) -> Result<OutputModules> {
        let chunks = (0..self.chunks.len())
            .into_par_iter()
            .map(|idx| self.emit_split_chunk(idx))
            .collect::<Result<Vec<SplitModule>>>()?;

        let modules = (0..self.split_points.len())
            .into_par_iter()
            .map(|idx| self.emit_split_module(idx))
            .collect::<Result<Vec<SplitModule>>>()?;

        // Emit the main module, consuming self since we're going to
        let main = self.emit_main_module()?;

        Ok(OutputModules {
            modules,
            chunks,
            main,
        })
    }

    /// Emit the main module.
    ///
    /// This will analyze the call graph and then perform some transformations on the module.
    /// - Clear out active segments that the split modules will initialize
    /// - Wipe away unused functions and data symbols
    /// - Re-export the memories, globals, and other items that the split modules will need
    /// - Convert the split module import functions to real functions that call the indirect function
    ///
    /// Once this is done, all the split module functions will have been removed, making the main module smaller.
    ///
    /// Emitting the main module is conceptually pretty simple. Emitting the split modules is more
    /// complex.
    fn emit_main_module(mut self) -> Result<SplitModule> {
        tracing::debug!("Emitting main bundle split module");

        // Perform some analysis of the module before we start messing with it
        let shared_funcs = self.main_shared_symbols();
        let unused_symbols = self.unused_main_symbols();

        assert!(unused_symbols.intersection(&shared_funcs).count() == 0);

        // Use the original module that contains all the right ids
        let mut out = std::mem::take(&mut self.source_module);

        // 1. Clear out the active segments that try to initialize functions for modules we just split off.
        //    When the side modules load, they will initialize functions into the table where the "holes" are.
        self.replace_segments_with_holes(&mut out, &unused_symbols);

        // 2. Wipe away the unused functions and data symbols
        let deleted = self.prune_main_symbols(&mut out, &unused_symbols)?;

        // 3. Change the functions called from split modules to be local functions that call the indirect function
        self.create_ifunc_table(&mut out);

        // 4. Re-export the memories, globals, and other stuff
        self.re_export_items(&mut out);

        // 5. Re-export shared functions
        self.re_export_functions(&mut out, &shared_funcs);

        // 6. Remove the reloc and linking custom sections
        self.remove_custom_sections(&mut out);

        let used = Used::new(&out, &deleted);

        // 7. Run the garbage collector to remove unused functions
        walrus::passes::gc::run(&mut out);

        Ok(SplitModule {
            module_name: "main".to_string(),
            component_name: None,
            bytes: out.emit_wasm(),
            relies_on_chunks: Default::default(),
            hash_id: None,
        })
    }

    /// Write the contents of the split modules to the output
    fn emit_split_module(&self, split_idx: usize) -> Result<SplitModule> {
        let split = self.split_points[split_idx].clone();

        // These are the symbols that will only exist in this module and not in the main module.
        let mut unique_symbols = split
            .reachable_graph
            .difference(&self.main_graph)
            .cloned()
            .collect::<HashSet<_>>();

        // The functions we'll need to import
        let mut symbols_to_import: HashSet<_> = split
            .reachable_graph
            .intersection(&self.main_graph)
            .cloned()
            .collect();

        // Identify the functions we'll delete
        let symbols_to_delete: HashSet<_> = self
            .main_graph
            .difference(&split.reachable_graph)
            .cloned()
            .collect();

        // Convert split chunk functions to imports
        let mut relies_on_chunks = HashSet::new();
        for (idx, chunk) in self.chunks.iter().enumerate() {
            let nodes_to_extract = unique_symbols
                .intersection(chunk)
                .cloned()
                .collect::<Vec<_>>();
            for node in nodes_to_extract {
                if !self.main_graph.contains(&node) {
                    // if let Node::Function(id) = node {
                    //     let func = self.source_module.funcs.get(id);
                    //     let name = func
                    //         .name
                    //         .as_ref()
                    //         .cloned()
                    //         .unwrap_or_else(|| format!("unknown - {}", id.index()));
                    //     tracing::debug!("Adding import for {name}");
                    // }

                    unique_symbols.remove(&node);
                    symbols_to_import.insert(node);
                    relies_on_chunks.insert(idx);
                }
            }
        }

        tracing::trace!(
            "Emitting module {}: {:?}",
            split.module_name,
            relies_on_chunks
        );

        // Remap the graph to our module's IDs
        let (mut out, ids_to_fns, _fns_to_ids) = parse_module_with_ids(&self.bindgened)?;
        let unique_symbols = self.remap_ids(&unique_symbols, &ids_to_fns);
        let symbols_to_delete = self.remap_ids(&symbols_to_delete, &ids_to_fns);
        let symbols_to_import = self.remap_ids(&symbols_to_import, &ids_to_fns);
        let split_export_func = ids_to_fns[self.fns_to_ids[&split.export_func]];

        // Do some basic cleanup of the module to make it smaller
        // This removes exports, imports, and the start function
        self.prune_split_module(&mut out);

        // Convert tables, memories, etc to imports rather than being locally defined
        self.convert_locals_to_imports(&mut out);

        // Clear away the data segments
        self.clear_data_segments(&mut out, &unique_symbols);

        // Clear out the element segments and then add in the initializers for the shared imports
        self.create_ifunc_initialzers(&mut out, &unique_symbols);

        // Take the symbols that are shared between the split modules and convert them to imports
        self.convert_shared_to_imports(&mut out, &symbols_to_import);

        // Convert our split module's functions to real functions that call the indirect function
        self.add_split_imports(&mut out, split.index, split_export_func, split.export_name);

        // Delete all the functions that are not reachable from the main module
        self.delete_main_funcs_from_split(&mut out, &symbols_to_delete);

        // Remove the reloc and linking custom sections
        self.remove_custom_sections(&mut out);

        // Run the gc to remove unused functions - also validates the module to ensure we can emit it properly
        walrus::passes::gc::run(&mut out);

        Ok(SplitModule {
            bytes: out.emit_wasm(),
            module_name: split.module_name.clone(),
            component_name: Some(split.component_name.clone()),
            relies_on_chunks,
            hash_id: Some(split.hash_name.clone()),
        })
    }

    /// Write a split chunk - this is a chunk with no special functions, just exports + initializers
    fn emit_split_chunk(&self, idx: usize) -> Result<SplitModule> {
        tracing::info!("emitting chunk {}", idx);

        let unique_symbols = &self.chunks[idx];

        // The functions we'll need to import
        let symbols_to_import: HashSet<_> = unique_symbols
            .intersection(&self.main_graph)
            .cloned()
            .collect();

        // Delete everything except the symbols that are reachable from this module
        let symbols_to_delete: HashSet<_> = self
            .main_graph
            .difference(&unique_symbols)
            .cloned()
            .collect();

        // We're going to export only the symbols that show up in other modules
        let mut symbols_to_export = HashSet::new();
        for sym in unique_symbols.iter() {
            for split in self.split_points.iter() {
                if split.reachable_graph.contains(sym) {
                    // if !self.main_graph.contains(sym) {
                    //     if let Node::Function(id) = sym {
                    //         let func = self.source_module.funcs.get(*id);
                    //         let name = func
                    //             .name
                    //             .as_ref()
                    //             .cloned()
                    //             .unwrap_or_else(|| format!("unknown - {}", id.index()));
                    //         // tracing::debug!("Adding export for {name}");
                    //     }
                    //     symbols_to_export.insert(*sym);
                    // }
                }
            }
        }

        // Make sure to remap any ids from the main module to this module
        let (mut out, ids_to_fns, _fns_to_ids) = parse_module_with_ids(&self.bindgened)?;
        let unique_symbols = self.remap_ids(unique_symbols, &ids_to_fns);
        let symbols_to_export = self.remap_ids(&symbols_to_export, &ids_to_fns);
        let symbols_to_import = self.remap_ids(&symbols_to_import, &ids_to_fns);
        let symbols_to_delete = self.remap_ids(&symbols_to_delete, &ids_to_fns);

        self.prune_split_module(&mut out);

        // Convert tables, memories, etc to imports rather than being locally defined
        self.convert_locals_to_imports(&mut out);

        // Clear away the data segments
        self.clear_data_segments(&mut out, &unique_symbols);

        // Clear out the element segments and then add in the initializers for the shared imports
        self.create_ifunc_initialzers(&mut out, &unique_symbols);

        // Take the symbols that are shared between the split modules and convert them to imports
        self.convert_shared_to_imports(&mut out, &symbols_to_import);

        // Re-export the re-exports
        self.re_export_functions(&mut out, &symbols_to_export);

        // We have to make sure our table matches that of the other tables even though we don't call them.
        self.expand_funcref_table_for_split(&mut out);

        // Make sure we haven't deleted anything important....
        self.delete_main_funcs_from_split(&mut out, &symbols_to_delete);

        // Remove the reloc and linking custom sections
        self.remove_custom_sections(&mut out);

        // Run the gc to remove unused functions - also validates the module to ensure we can emit it properly
        walrus::passes::gc::run(&mut out);

        Ok(SplitModule {
            bytes: out.emit_wasm(),
            module_name: "split".to_string(),
            component_name: None,
            relies_on_chunks: Default::default(),
            hash_id: None,
        })
    }

    fn expand_funcref_table_for_split(&self, out: &mut Module) {
        let ifunc_table_id = self.load_funcref_table(out);
        let _segment_start = self
            .expand_ifunc_table_max(out, ifunc_table_id, self.split_points.len())
            .expect("failed to expand ifunc table");
    }

    /// Convert any shared functions into imports
    fn convert_shared_to_imports(&self, out: &mut Module, symbols_to_import: &HashSet<Node>) {
        // let mut already_imported = HashSet::new();

        // for imp in out.imports.iter_mut() {
        //     if let ImportKind::Function(id) = imp.kind {
        //         already_imported.insert(id);
        //     }
        // }

        for symbol in symbols_to_import {
            if let Node::Function(id) = *symbol {
                let func = out.funcs.get_mut(id);
                // let Some(name) = func.name.clone() else {
                //     continue;
                // };
                // .unwrap_or_else(|| format!("unknown - {}", id.index()));

                // if already_imported.contains(&id) {
                //     // tracing::error!("Already imported: {:?}", name);
                //     continue;
                // }

                if let Some(name) = func.name.clone() {
                    let name = format!("__exported_{name}");
                    let ty = func.ty();
                    let import = out
                        .imports
                        .add("__wasm_split", &name, ImportKind::Function(id));
                    let func = out.funcs.get_mut(id);
                    func.kind = FunctionKind::Import(ImportedFunction { import, ty });
                }
            }
        }
    }

    /// Convert split import functions to local functions that call an indirect function that will
    /// be filled in from the loaded split module.
    ///
    /// This is because these imports are going to be delayed until the split module is loaded
    /// and loading in the main module these as imports won't be possible since the imports won't
    /// be resolved until the split module is loaded.
    fn create_ifunc_table(&self, out: &mut Module) {
        let ifunc_table = self.load_funcref_table(out);
        let dummy_func = self.make_dummy_func(out);

        out.exports.add("__indirect_function_table", ifunc_table);

        // Expand the ifunc table to accomodate the new ifuncs
        let segment_start = self
            .expand_ifunc_table_max(out, ifunc_table, self.split_points.len())
            .expect("failed to expand ifunc table");

        // Delete the split import functions and replace them with local functions
        //
        // Start by pushing all the shared imports into the list
        // These don't require an additional stub function
        let mut ifuncs = vec![];

        // Push the split import functions into the list - after we've pushed in the shared imports
        for idx in 0..self.split_points.len() {
            // this is okay since we're in the main module
            let import_func = self.split_points[idx].import_func;
            let import_id = self.split_points[idx].import_id;
            let ty_id = out.funcs.get(import_func).ty();
            let stub_idx = segment_start + ifuncs.len();

            // Replace the import function with a local function that calls the indirect function
            out.funcs.get_mut(import_func).kind =
                self.make_stub_funcs(out, ifunc_table, ty_id, stub_idx as _);

            // And remove the corresponding import
            out.imports.delete(import_id);

            // Push into the list the properly typed dummy func so the entry is populated
            // unclear if the typing is important here
            ifuncs.push(dummy_func);
        }

        // Now add segments to the ifunc table
        out.tables
            .get_mut(ifunc_table)
            .elem_segments
            .insert(out.elements.add(
                ElementKind::Active {
                    table: ifunc_table,
                    offset: ConstExpr::Value(ir::Value::I32(segment_start as _)),
                },
                ElementItems::Functions(ifuncs),
            ));
    }

    /// Re-export the memories, globals, and other items from the main module to the side modules
    fn re_export_items(&self, out: &mut Module) {
        // Re-export memories
        for (idx, memory) in out.memories.iter().enumerate() {
            let name = memory
                .name
                .clone()
                .unwrap_or_else(|| format!("__memory_{}", idx));
            out.exports.add(&name, memory.id());
        }

        // Re-export globals
        for (idx, global) in out.globals.iter().enumerate() {
            let global_name = format!("__global__{idx}");
            out.exports.add(&global_name, global.id());
        }

        // Export any tables
        for (idx, table) in out.tables.iter().enumerate() {
            if table.element_ty != RefType::Funcref {
                let table_name = format!("__imported_table_{}", idx);
                out.exports.add(&table_name, table.id());
            }
        }
    }

    fn exported_functions(&self) -> HashSet<FunctionId> {
        self.source_module
            .exports
            .iter()
            .filter_map(|e| match e.item {
                ExportItem::Function(id) => Some(id),
                _ => None,
            })
            .collect()
    }

    fn re_export_functions(&self, out: &mut Module, funcs: &HashSet<Node>) {
        // Make sure to re-export any shared functions.
        // This is somewhat in-efficient because it's re-exporting symbols that don't need to be re-exported.
        // We could just try walking the code looking for directly called functions, but that's a bit more complex.
        for func_id in funcs.iter().copied() {
            if let Node::Function(func_id) = func_id {
                if let Some(name) = out.funcs.get(func_id).name.as_ref().cloned() {
                    out.exports.add(&format!("__exported_{}", name), func_id);
                }
            }
        }
    }

    fn prune_main_symbols(
        &self,
        out: &mut Module,
        unused_symbols: &HashSet<Node>,
    ) -> Result<HashSet<FunctionId>> {
        // Wipe the split point exports
        for split in self.split_points.iter() {
            // it's okay that we're not re-mapping IDs since this is just used by the main module
            out.exports.delete(split.export_id);
        }

        let mut deleted = HashSet::new();

        // And then any actual symbols from the callgraph
        for symbol in unused_symbols.iter().cloned() {
            match symbol {
                // Simply delete functions
                Node::Function(id) => {
                    out.funcs.delete(id);
                    deleted.insert(id);
                }

                // Otherwise, zero out the data segment, which should lead to elimination by wasm-opt
                Node::DataSymbol(id) => {
                    let symbol = self
                        .data_symbols
                        .get(&id)
                        .context("Failed to find data symbol")?;

                    // VERY IMPORTANT
                    //
                    // apparently wasm-bindgen makes data segments that aren't the main one
                    // even *touching* those will break the vtable / binding layer
                    // We can only interact with the first data segment - the rest need to stay avaiable
                    // for the `.js` to interact with.
                    if symbol.which_data_segment == 0 {
                        let data_id = out.data.iter().nth(symbol.which_data_segment).unwrap().id();
                        let data = out.data.get_mut(data_id);
                        for i in symbol.segment_offset..symbol.segment_offset + symbol.symbol_size {
                            data.value[i] = 0;
                        }
                    }
                }
            }
        }

        Ok(deleted)
    }

    // 2.1 Create a dummy func that will be overridden later as modules pop in
    // 2.2 swap the segment entries with the dummy func, leaving hole in its placed that will be filled in later
    fn replace_segments_with_holes(&self, out: &mut Module, unused_symbols: &HashSet<Node>) {
        let dummy_func = self.make_dummy_func(out);
        for element in out.elements.iter_mut() {
            match &mut element.items {
                ElementItems::Functions(vec) => {
                    for item in vec.iter_mut() {
                        if unused_symbols.contains(&Node::Function(*item)) {
                            *item = dummy_func;
                        }
                    }
                }
                ElementItems::Expressions(_ref_type, const_exprs) => {
                    for item in const_exprs.iter_mut() {
                        if let &mut ConstExpr::RefFunc(id) = item {
                            if unused_symbols.contains(&Node::Function(id)) {
                                *item = ConstExpr::RefFunc(dummy_func);
                            }
                        }
                    }
                }
            }
        }
    }

    fn create_ifunc_initialzers(&self, out: &mut Module, unique_symbols: &HashSet<Node>) {
        let ifunc_table = self.load_funcref_table(out);

        let mut initializers = HashMap::new();
        for segment in out.elements.iter_mut() {
            let ElementKind::Active { offset, .. } = &mut segment.kind else {
                continue;
            };

            let ConstExpr::Value(ir::Value::I32(offset)) = offset else {
                continue;
            };

            match &segment.items {
                ElementItems::Functions(vec) => {
                    for (idx, id) in vec.into_iter().enumerate() {
                        if unique_symbols.contains(&Node::Function(*id)) {
                            initializers
                                .insert(*offset + idx as i32, ElementItems::Functions(vec![*id]));
                        }
                    }
                }

                ElementItems::Expressions(ref_type, const_exprs) => {
                    for (idx, expr) in const_exprs.iter().enumerate() {
                        if let ConstExpr::RefFunc(id) = expr {
                            if unique_symbols.contains(&Node::Function(*id)) {
                                initializers.insert(
                                    *offset + idx as i32,
                                    ElementItems::Expressions(
                                        *ref_type,
                                        vec![ConstExpr::RefFunc(*id)],
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }

        // Wipe away references to these segments
        for table in out.tables.iter_mut() {
            table.elem_segments.clear();
        }

        // Wipe away the element segments themselves
        let segments_to_delete: Vec<_> = out.elements.iter().map(|e| e.id()).collect();
        for id in segments_to_delete {
            out.elements.delete(id);
        }

        // Add in our new segments
        let ifunc_table_ = out.tables.get_mut(ifunc_table);
        for (offset, items) in initializers {
            let kind = ElementKind::Active {
                table: ifunc_table,
                offset: ConstExpr::Value(ir::Value::I32(offset)),
            };

            ifunc_table_
                .elem_segments
                .insert(out.elements.add(kind, items));
        }
    }

    fn add_split_imports(
        &self,
        out: &mut Module,
        split_idx: usize,
        split_export_func: FunctionId,
        split_export_name: String,
    ) {
        let ifunc_table_id = self.load_funcref_table(out);
        let segment_start = self
            .expand_ifunc_table_max(out, ifunc_table_id, self.split_points.len())
            .unwrap();

        // Make sure to re-export the split func
        out.exports.add(&split_export_name, split_export_func);

        // Add the elements back to the table
        out.tables
            .get_mut(ifunc_table_id)
            .elem_segments
            .insert(out.elements.add(
                ElementKind::Active {
                    table: ifunc_table_id,
                    offset: ConstExpr::Value(ir::Value::I32((segment_start + split_idx) as i32)),
                },
                ElementItems::Functions(vec![split_export_func]),
            ));
    }

    fn delete_main_funcs_from_split(&self, out: &mut Module, symbols_to_delete: &HashSet<Node>) {
        let mut deleted = HashSet::new();
        for node in symbols_to_delete {
            if let Node::Function(id) = *node {
                deleted.insert(id);
            }
        }

        let used = wasm_used::Used::new(&out, &deleted);

        for node in symbols_to_delete {
            if let Node::Function(id) = *node {
                out.funcs.delete(id);
            }
        }
    }

    fn prune_split_module(&self, out: &mut Module) {
        // Clear the module's start/main
        if let Some(start) = out.start.take() {
            if let Some(export) = out.exports.get_exported_func(start) {
                out.exports.delete(export.id());
            }
        }

        // We're going to import the funcref table, so wipe it altogether
        for table in out.tables.iter_mut() {
            table.elem_segments.clear();
        }

        // Wipe all our imports - we're going to use a different set of imports
        let all_imports: HashSet<_> = out.imports.iter().map(|i| i.id()).collect();
        for import_id in all_imports {
            out.imports.delete(import_id);
        }

        // Wipe away memories
        let all_memories: Vec<_> = out.memories.iter().map(|m| m.id()).collect();
        for memory_id in all_memories {
            out.memories.get_mut(memory_id).data_segments.clear();
        }
    }

    fn make_dummy_func(&self, out: &mut Module) -> FunctionId {
        let mut b = FunctionBuilder::new(&mut out.types, &[], &[]);
        b.name("dummy".into()).func_body().unreachable();
        b.finish(vec![], &mut out.funcs)
    }

    fn convert_locals_to_imports(&self, out: &mut Module) {
        // Add exports that call the corresponding import
        let exports = out.exports.iter().map(|e| e.id()).collect::<Vec<_>>();
        for export_id in exports {
            out.exports.delete(export_id);
            // let export = out.exports.get_mut(export_id);

            // // for main + our custom splits, just wipe them
            // if export.name == "main" || export.name.contains("__wasm_split") {
            //     out.exports.delete(export_id);
            //     continue;
            // }

            // // Otherwise, we might actually have code that *calls* these exports
            // // Let's transform them to imports
            // if let ExportItem::Function(id) = export.item {
            //     let func = out.funcs.get_mut(id);
            //     let name = export.name.clone();

            //     // Delete the export
            //     out.exports.delete(export_id);

            //     // And convert the function to an import
            //     let import = out.imports.add("__wasm_split", &name, id);
            //     func.kind = FunctionKind::Import(ImportedFunction {
            //         import,
            //         ty: func.ty(),
            //     });
            // }
            // }
        }

        // // Also convert the extra symbols to imports
        // for extra in self.extra_symbols.iter() {
        //     if let Node::Function(id) = extra {
        //         let func = out.funcs.get_mut(*id);
        //         let Some(name) = func.name.clone() else {
        //             continue;
        //         };

        //         let import = out.imports.add("__wasm_split", &name, *id);
        //         func.kind = FunctionKind::Import(ImportedFunction {
        //             import,
        //             ty: func.ty(),
        //         });
        //     }
        // }

        // Convert the tables to imports.
        // Should be as simple as adding a new import and then writing the `.import` field
        for (idx, table) in out.tables.iter_mut().enumerate() {
            let name = table.name.clone().unwrap_or_else(|| {
                if table.element_ty == RefType::Funcref {
                    format!("__indirect_function_table")
                } else {
                    format!("__imported_table_{}", idx)
                }
            });
            let import = out.imports.add("__wasm_split", &name, table.id());
            table.import = Some(import);
        }

        // Convert the memories to imports
        // Should be as simple as adding a new import and then writing the `.import` field
        for (idx, memory) in out.memories.iter_mut().enumerate() {
            let name = memory
                .name
                .clone()
                .unwrap_or_else(|| format!("__memory_{}", idx));
            let import = out.imports.add("__wasm_split", &name, memory.id());
            memory.import = Some(import);
        }

        // Convert the globals to imports
        // We might not use the global, so if we don't, we can just get
        let global_ids: Vec<_> = out.globals.iter().map(|t| t.id()).collect();
        for (idx, global_id) in global_ids.into_iter().enumerate() {
            let global = out.globals.get_mut(global_id);
            let global_name = format!("__global__{idx}");
            let import = out.imports.add("__wasm_split", &global_name, global.id());
            global.kind = GlobalKind::Import(import);
        }
    }

    fn clear_data_segments(&self, out: &mut Module, unique_symbols: &HashSet<Node>) {
        // Preserve the data symbols for this module and then clear them away
        let data_ids: Vec<_> = out.data.iter().map(|t| t.id()).collect();
        for (idx, data_id) in data_ids.into_iter().enumerate() {
            let data = out.data.get_mut(data_id);

            // Take the data out of the vec - zeroing it out unless we patch it in manually
            let contents = data.value.split_off(0);

            // Zero out the non-primary data segments
            if idx != 0 {
                continue;
            }

            let DataKind::Active { memory, offset } = data.kind else {
                continue;
            };

            let ConstExpr::Value(ir::Value::I32(data_offset)) = offset else {
                continue;
            };

            // And then assign chunks of the data to new data entries that will override the individual slots
            for unique in unique_symbols {
                if let Node::DataSymbol(id) = unique {
                    let symbol = self.data_symbols.get(&id).expect("missing data symbol");
                    if symbol.which_data_segment == idx {
                        let range =
                            symbol.segment_offset..symbol.segment_offset + symbol.symbol_size;
                        let offset = ConstExpr::Value(ir::Value::I32(
                            data_offset + symbol.segment_offset as i32,
                        ));
                        out.data.add(
                            DataKind::Active { memory, offset },
                            contents[range].to_vec(),
                        );
                    }
                }
            }
        }
    }

    /// Load the funcref table from the main module. This *should* exist for all modules created by
    /// Rustc or Wasm-Bindgen, but we create it if it doesn't exist.
    fn load_funcref_table(&self, out: &mut Module) -> TableId {
        let ifunc_table = out
            .tables
            .iter()
            .find(|t| t.element_ty == RefType::Funcref)
            .map(|t| t.id());

        if let Some(table) = ifunc_table {
            table
        } else {
            out.tables.add_local(false, 0, None, RefType::Funcref)
        }
    }

    /// Convert the imported function to a local function that calls an indirect function from the table
    ///
    /// This will enable the main module (and split modules) to call functions from outside their own module.
    /// The functions might not exist when the main module is loaded, so we'll register some elements
    /// that fill those in eventually.
    fn make_stub_funcs(
        &self,
        out: &mut Module,
        table: TableId,
        ty_id: TypeId,
        table_idx: i32,
    ) -> FunctionKind {
        // Convert the import function to a local function that calls the indirect function from the table
        let ty = out.types.get(ty_id);

        let params = ty.params().to_vec();
        let results = ty.results().to_vec();
        let args: Vec<_> = params.iter().map(|ty| out.locals.add(*ty)).collect();

        // New function that calls the indirect function
        let mut builder = FunctionBuilder::new(&mut out.types, &params, &results);
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
            table,
        }));

        FunctionKind::Local(builder.local_func(args))
    }

    /// Expand the ifunc table to accomodate the new ifuncs
    ///
    /// returns the old maximum
    fn expand_ifunc_table_max(
        &self,
        out: &mut Module,
        table: TableId,
        num_ifuncs: usize,
    ) -> Option<usize> {
        let ifunc_table_ = out.tables.get_mut(table);

        if let Some(max) = ifunc_table_.maximum {
            ifunc_table_.maximum = Some(max + num_ifuncs as u64);
            ifunc_table_.initial += num_ifuncs as u64;
            return Some(max as usize);
        }

        None
    }

    // only keep the target-features and names section so wasm-opt can use it to optimize the output
    fn remove_custom_sections(&self, out: &mut Module) {
        let sections_to_delete = out
            .customs
            .iter()
            .filter_map(|(id, section)| {
                if section.name() == "target_features" {
                    None
                } else {
                    Some(id)
                }
            })
            .collect::<Vec<_>>();

        for id in sections_to_delete {
            out.customs.delete(id);
        }
    }

    /// Use the Louvain algorithm (okay not actually, is just greedy right now)
    /// to determine communties in the split modules so we can create  efficient chunks
    ///
    /// Todo: we could chunk up the main module itself! Not going to now but it would enable parallel downloads of the main chunk
    fn build_split_chunks(&mut self) {
        // Every symbol and the chunks that use it
        // We're only going to try optimizing functions used across multiple chunks
        let mut funcs_used_by_chunks: HashMap<Node, HashSet<usize>> = HashMap::new();
        for split in self.split_points.iter() {
            for item in split.reachable_graph.iter() {
                if self.main_graph.contains(item) {
                    continue;
                }

                funcs_used_by_chunks
                    .entry(item.clone())
                    .or_default()
                    .insert(split.index);
            }
        }

        for import in self.source_module.imports.iter() {
            tracing::debug!("import: {:?}", import.name);
        }

        for export in self.source_module.exports.iter() {
            tracing::warn!("export: {:?}", export.name);
        }

        for extra in self.extra_symbols.iter() {
            if let Node::Function(id) = extra {
                let name = self.source_module.funcs.get(*id).name.as_ref().unwrap();
                tracing::info!("extra: {:?}", name);
            }
        }

        // Remove all the symbols that are only used by one module
        funcs_used_by_chunks.retain(|_, v| v.len() > 1);

        let mut roots = funcs_used_by_chunks.keys().cloned().collect::<HashSet<_>>();

        for export in self.exported_functions() {
            roots.insert(Node::Function(export));
        }

        for import in self.source_module.imports.iter() {
            if let ImportKind::Function(id) = import.kind {
                roots.insert(Node::Function(id));
            }
        }

        // for extra in self.extra_symbols.iter() {
        //     roots.insert(*extra);
        // }

        let mut reachable = reachable_graph(&self.call_graph, &roots);
        // reachable.retain(|k| !self.main_graph.contains(k));

        // // Create the roots
        // let mut roots = funcs_used_by_chunks.keys().cloned().collect::<HashSet<_>>();

        // let exports = self
        //     .source_module
        //     .exports
        //     .iter()
        //     .flat_map(|e| match e.item {
        //         ExportItem::Function(id) => Some(Node::Function(id)),
        //         _ => None,
        //     })
        //     .collect::<HashSet<_>>();
        // let export_call_graph = make_call_graph(&self.call_graph, &exports);

        // 10k symbols can lead to almost any chunk size (mb)
        const MAX_CHUNK_SIZE: usize = 10000;
        let mut remaining_functions: BTreeSet<Node> = reachable.into_iter().collect();

        while !remaining_functions.is_empty() {
            let current_func = remaining_functions.pop_last().unwrap();
            let mut current_chunk = HashSet::new();
            current_chunk.insert(current_func.clone());
            remaining_functions.remove(&current_func);

            let mut removes = vec![];

            for func in remaining_functions.iter().copied() {
                if current_chunk.len() >= MAX_CHUNK_SIZE {
                    break;
                }

                let is_child = self
                    .call_graph
                    .get(&current_func)
                    .map(|children| children.contains(&func))
                    .unwrap_or_default();
                let is_parent = self
                    .parent_graph
                    .get(&current_func)
                    .map(|parents| parents.contains(&func))
                    .unwrap_or_default();

                if is_child || is_parent {
                    removes.push(func);
                }
            }

            for remove in removes {
                current_chunk.insert(remove);
                remaining_functions.remove(&remove);
            }

            self.chunks.push(current_chunk);
        }

        // Further optimize chunks if needed
        // Merge small chunks if possible
        // todo: make this a ratio of the total size of all chunks - we don't want too many chunks (maybe only like max 1:10?)
        // we would need to measure the size of each chunk
        let mut i = 0;
        while i < self.chunks.len() {
            let min_chunk_size = (MAX_CHUNK_SIZE / 2).max(40);
            if self.chunks[i].len() < min_chunk_size / 2 {
                let mut best_merge = None;
                let mut min_size = usize::MAX;

                for j in (i + 1)..self.chunks.len() {
                    let merged_size = self.chunks[i].len() + self.chunks[j].len();
                    if merged_size <= MAX_CHUNK_SIZE && merged_size < min_size {
                        best_merge = Some(j);
                        min_size = merged_size;
                    }
                }

                if let Some(j) = best_merge {
                    let chunk_j = self.chunks.remove(j);
                    self.chunks[i].extend(chunk_j);
                    continue;
                }
            }
            i += 1;
        }
    }

    /// Get the symbols that are shared between the main module and the split modules
    ///
    /// This collects *all* the symbols even if they are not called from main (only transitively).
    fn main_shared_symbols(&self) -> HashSet<Node> {
        let mut shared_funcs = HashSet::new();

        for split in self.split_points.iter() {
            shared_funcs.extend(split.reachable_graph.iter());
        }

        shared_funcs.retain(|sym| !self.main_graph.contains(sym));

        for injected in self.extra_symbols.iter() {
            shared_funcs.insert(*injected);
        }

        for import in self.source_module.imports.iter() {
            if let ImportKind::Function(id) = import.kind {
                shared_funcs.insert(Node::Function(id));
            }
        }

        shared_funcs
    }

    fn unused_main_symbols(&self) -> HashSet<Node> {
        let mut unique = HashSet::new();

        for split in self.split_points.iter() {
            unique.extend(split.reachable_graph.iter());
        }

        unique.retain(|sym| !self.main_graph.contains(sym));

        let mut fix_expots = vec![];
        for _u in unique.iter() {
            if let Node::Function(_u) = _u {
                if self.source_module.exports.get_exported_func(*_u).is_some() {
                    fix_expots.push(*_u);
                }
            }
        }

        for _u in fix_expots.iter() {
            tracing::warn!("fixing export: {:?}", _u);
            unique.remove(&Node::Function(*_u));
        }

        unique
    }

    /// Accumulate the relocations from the original module, create a relocation map, and then convert
    /// that to our *new* module's symbols.
    fn build_call_graph(&mut self) -> Result<()> {
        let original = ModuleWithRelocations::new(&self.original)?;

        // map of old to new
        let mut f_to_f = HashMap::new();

        let mut names = original.names_to_funcs.clone();
        for func in self.source_module.funcs.iter() {
            let Some(name) = func.name.as_ref() else {
                continue;
            };

            match names.remove(name) {
                Some(original_id) => {
                    f_to_f.insert(original_id, func.id());
                }
                None => {
                    self.extra_symbols.insert(Node::Function(func.id()));
                    tracing::error!("func not found: {:?}", name)
                }
            }
        }

        // these are the ones that wasm-bindgen killed
        for (name, id) in names {
            tracing::info!("func remaining: {:?}", name);
        }

        let find_old = |old: &Node| -> Option<Node> {
            match old {
                Node::Function(id) => f_to_f.get(id).map(|id| Node::Function(*id)),
                Node::DataSymbol(idx) => Some(Node::DataSymbol(*idx)),
            }
        };

        for (old, children) in original.call_graph.iter() {
            let mut new_children = HashSet::new();
            for child in children {
                if let Some(new) = find_old(child) {
                    new_children.insert(new);
                } else {
                    if let Node::Function(id) = child {
                        let name = original.module.funcs.get(*id).name.as_ref();
                        tracing::error!("func not found: {:?}", name);
                    }

                    // tracing::error!("func not found: {:?}", child);
                }
            }

            if let Some(new) = find_old(old) {
                new_children.insert(new);
            } else {
                if let Node::Function(id) = old {
                    let name = original.module.funcs.get(*id).name.as_ref();
                    tracing::error!("func not found: {:?}", name);
                }
            }
        }

        // Fill in the parent graph
        for (parnet, children) in self.call_graph.iter() {
            for child in children {
                self.parent_graph.entry(*child).or_default().insert(*parnet);
            }
        }

        // Now go fill in the reachability graph for each of the split points
        //
        // If we don't use the imports/exports they will be pruned by the gc pass, so it's okay
        // being a little too liberal here.
        self.split_points.iter_mut().for_each(|split| {
            let mut roots: HashSet<_> = [Node::Function(split.export_func)].into();

            // Addin the extern shim functions
            for extra in self.extra_symbols.iter() {
                roots.insert(*extra);
            }

            // Make sure the imports are counted too
            for import in self.source_module.imports.iter() {
                if let ImportKind::Function(id) = import.kind {
                    roots.insert(Node::Function(id));
                }
            }

            split.reachable_graph = reachable_graph(&self.call_graph, &roots);
        });

        // And then the reachability graph for main
        self.main_graph = reachable_graph(&self.call_graph, &self.main_roots());

        Ok(())
    }

    fn main_roots(&self) -> HashSet<Node> {
        // Accumulate all the split entrypoints
        // This will include wasm_bindgen functions too
        let exported_splits = self
            .split_points
            .iter()
            .map(|f| f.export_func)
            .collect::<HashSet<_>>();

        // And only return the functions that are reachable from the main module's start function
        let mut roots = self
            .source_module
            .exports
            .iter()
            .filter_map(|e| match e.item {
                ExportItem::Function(id) if !exported_splits.contains(&id) => {
                    Some(Node::Function(id))
                }
                _ => None,
            })
            .chain(self.source_module.start.map(|f| Node::Function(f)))
            .collect::<HashSet<Node>>();

        // Make sure the extern shim functions are counted too
        for extra in self.extra_symbols.iter() {
            roots.insert(*extra);
        }

        // Also add "imports" to the roots
        for import in self.source_module.imports.iter() {
            if let ImportKind::Function(id) = import.kind {
                roots.insert(Node::Function(id));
            }
        }

        roots
    }

    /// Convert this set of nodes to reference the new module
    fn remap_ids(&self, set: &HashSet<Node>, ids_to_fns: &[FunctionId]) -> HashSet<Node> {
        let mut out = HashSet::with_capacity(set.len());

        for node in set {
            match node {
                // Remap the function IDs
                Node::Function(id) => out.insert(Node::Function(ids_to_fns[self.fns_to_ids[&id]])),
                // data symbols don't need remapping
                Node::DataSymbol(id) => out.insert(Node::DataSymbol(*id)),
            };
        }

        out
    }
}

/// Parse a module and return the mapping of index to FunctionID.
/// We'll use this mapping to remap ModuleIDs
fn parse_module_with_ids(
    bindgened: &[u8],
) -> Result<(Module, Vec<FunctionId>, HashMap<FunctionId, usize>)> {
    let ids = Arc::new(RwLock::new(Vec::new()));
    let ids_ = ids.clone();
    let module = Module::from_buffer_with_config(
        &bindgened,
        &ModuleConfig::new().on_parse(move |_m, our_ids| {
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

    Ok((module, ids, fns_to_ids))
}

struct ModuleWithRelocations<'a> {
    module: Module,
    symbols: Vec<SymbolInfo<'a>>,
    names_to_funcs: HashMap<String, FunctionId>,
    call_graph: HashMap<Node, HashSet<Node>>,
    parents: HashMap<Node, HashSet<Node>>,
    relocation_map: HashMap<Node, Vec<RelocationEntry>>,
    data_symbols: BTreeMap<usize, DataSymbol>,
    data_section_range: Range<usize>,
}

impl<'a> ModuleWithRelocations<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self> {
        let module = Module::from_buffer(bytes)?;
        let raw_data = parse_bytes_to_data_segment(&bytes)?;
        let names_to_funcs = module
            .funcs
            .iter()
            .enumerate()
            .flat_map(|(idx, f)| Some((f.name.as_ref()?.clone(), f.id())))
            .collect();

        let mut module = Self {
            module,
            data_symbols: raw_data.data_symbols,
            data_section_range: raw_data.data_range,
            symbols: raw_data.symbols,
            names_to_funcs,
            call_graph: Default::default(),
            relocation_map: Default::default(),
            parents: Default::default(),
        };

        module.build_code_call_graph()?;
        module.build_data_call_graph()?;

        for (func, children) in module.call_graph.iter() {
            for child in children {
                module.parents.entry(*child).or_default().insert(*func);
            }
        }

        Ok(module)
    }

    fn build_code_call_graph(&mut self) -> Result<()> {
        let codes_relocations = self.collect_relocations_from_section("reloc.CODE")?;
        let mut relocations = codes_relocations.iter().peekable();

        for (func_id, local) in self.module.funcs.iter_local() {
            let range = local
                .original_range
                .clone()
                .context("local function has no range")?;

            // Walk with relocation
            while let Some(entry) =
                relocations.next_if(|entry| entry.relocation_range().start < range.end)
            {
                let reloc_range = entry.relocation_range();
                assert!(reloc_range.start >= range.start);
                assert!(reloc_range.end <= range.end);

                if let Some(target) = self.get_symbol_dep_node(entry.index as usize)? {
                    let us = Node::Function(func_id);
                    self.call_graph.entry(us).or_default().insert(target);
                    self.relocation_map.entry(us).or_default().push(*entry);
                }
            }
        }

        assert!(relocations.next().is_none());

        Ok(())
    }

    fn build_data_call_graph(&mut self) -> Result<()> {
        let data_relocations = self.collect_relocations_from_section("reloc.DATA")?;
        let mut relocations = data_relocations.iter().peekable();

        let symbols_sorted = self
            .data_symbols
            .iter()
            .map(|(_, sym)| sym)
            .sorted_by(|a, b| a.range.start.cmp(&b.range.start));

        for symbol in symbols_sorted {
            let start = symbol.range.start - self.data_section_range.start;
            let end = symbol.range.end - self.data_section_range.start;
            let range = start..end;

            while let Some(entry) =
                relocations.next_if(|entry| entry.relocation_range().start < range.end)
            {
                let reloc_range = entry.relocation_range();
                assert!(reloc_range.start >= range.start);
                assert!(reloc_range.end <= range.end);

                if let Some(target) = self.get_symbol_dep_node(entry.index as usize)? {
                    let dep = Node::DataSymbol(symbol.index);
                    self.call_graph.entry(dep).or_default().insert(target);
                    self.relocation_map.entry(dep).or_default().push(*entry);
                }
            }
        }

        assert!(relocations.next().is_none());

        Ok(())
    }

    /// Accumulate all relocations from a section.
    ///
    /// Parses the section using the RelocSectionReader and returns a vector of relocation entries.
    fn collect_relocations_from_section(&self, name: &str) -> Result<Vec<RelocationEntry>> {
        let (_reloc_id, code_reloc) = self
            .module
            .customs
            .iter()
            .find(|(_, c)| c.name() == name)
            .context("Module does not contain the reloc section")?;

        let code_reloc_data = code_reloc.data(&Default::default());
        let relocations = RelocSectionReader::new(&code_reloc_data, 0)
            .context("failed to parse reloc section")?
            .entries()
            .into_iter()
            .flatten()
            .collect();

        Ok(relocations)
    }

    /// Get the symbol's corresponding entry in the call graph
    ///
    /// This might panic if the source module isn't built properly. Make sure to enable LTO and `--emit-relocs`
    /// when building the source module.
    fn get_symbol_dep_node(&self, index: usize) -> Result<Option<Node>> {
        let res = match self.symbols[index] {
            SymbolInfo::Data { .. } => Some(Node::DataSymbol(index)),
            SymbolInfo::Func { name, .. } => Some(Node::Function(
                *self
                    .names_to_funcs
                    .get(name.expect("local func symbol without name?"))
                    .unwrap(),
            )),

            _ => None,
        };

        Ok(res)
    }
}

#[derive(Debug, Clone)]
pub struct SplitPoint {
    module_name: String,
    import_id: ImportId,
    export_id: ExportId,
    import_func: FunctionId,
    export_func: FunctionId,
    component_name: String,
    index: usize,
    reachable_graph: HashSet<Node>,
    hash_name: String,

    #[allow(unused)]
    import_name: String,

    #[allow(unused)]
    export_name: String,
}

/// Search the module's imports and exports for functions marked as split points.
///
/// These will be in the form of:
///
/// __wasm_split_00<module>00_<import|export>_<hash>_<function>
///
/// For a function named `SomeRoute2` in the module `add_body_element`, the pairings would be:
///
/// __wasm_split_00add_body_element00_import_abef5ee3ebe66ff17677c56ee392b4c2_SomeRoute2
/// __wasm_split_00add_body_element00_export_abef5ee3ebe66ff17677c56ee392b4c2_SomeRoute2
///
fn accumulate_split_points(module: &Module) -> Vec<SplitPoint> {
    let mut index = 0;

    module
        .imports
        .iter()
        .sorted_by(|a, b| a.name.cmp(&b.name))
        .flat_map(|import| {
            if !import.name.starts_with("__wasm_split_00") {
                return None;
            }

            let ImportKind::Function(import_func) = import.kind else {
                return None;
            };

            // Parse the import name to get the module name, the hash, and the function name
            let remain = import.name.trim_start_matches("__wasm_split_00___");
            let (module_name, rest) = remain.split_once("___00").unwrap();
            let (hash, fn_name) = rest.trim_start_matches("_import_").split_once("_").unwrap();

            // Look for the export with the same name
            let export_name =
                format!("__wasm_split_00___{module_name}___00_export_{hash}_{fn_name}");
            let export_func = module
                .exports
                .get_func(&export_name)
                .expect("Could not find export");
            let export = module.exports.get_exported_func(export_func).unwrap();

            let our_index = index;
            index += 1;

            Some(SplitPoint {
                export_id: export.id(),
                import_id: import.id(),
                module_name: module_name.to_string(),
                import_name: import.name.clone(),
                import_func,
                export_func,
                export_name,
                hash_name: hash.to_string(),
                component_name: fn_name.to_string(),
                index: our_index,
                reachable_graph: Default::default(),
            })
        })
        .collect()
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, PartialOrd, Ord, Clone)]
pub enum Node {
    Function(FunctionId),
    DataSymbol(usize),
}

fn reachable_graph(deps: &HashMap<Node, HashSet<Node>>, roots: &HashSet<Node>) -> HashSet<Node> {
    let mut queue: VecDeque<Node> = roots.iter().copied().collect();
    let mut reachable = HashSet::<Node>::new();
    let mut parents = HashMap::<Node, Node>::new();

    while let Some(node) = queue.pop_front() {
        reachable.insert(node);
        let Some(children) = deps.get(&node) else {
            continue;
        };
        for child in children {
            if reachable.contains(&child) {
                continue;
            }
            parents.entry(*child).or_insert(node);
            queue.push_back(*child);
        }
    }

    reachable
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
    name: String,
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
                let reader = LinkingSectionReader::new(section.data(), 0)?;
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
        let SymbolInfo::Data {
            symbol: Some(symbol),
            name,
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
                index: index as usize,
                range,
                name: name.to_string(),
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
