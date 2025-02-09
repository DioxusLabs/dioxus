use anyhow::{Context, Result};
use itertools::Itertools;
use rayon::prelude::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    hash::Hash,
    ops::Range,
    sync::{Arc, RwLock},
};
use used::Used;
use walrus::{
    ir, ConstExpr, DataKind, ElementItems, ElementKind, ExportId, ExportItem, FunctionBuilder,
    FunctionId, FunctionKind, GlobalKind, IdsToIndices, ImportId, ImportKind, ImportedFunction,
    Module, ModuleConfig, RefType, TableId, TypeId,
};
use wasmparser::{
    Linking, LinkingSectionReader, Payload, RelocSectionReader, RelocationEntry, SymbolInfo,
};

mod used;

pub const MAKE_LOAD_JS: &'static str = include_str!("./__wasm_split.js");

/// A parsed wasm module with additional metadata and functionality for splitting and patching.
///
/// This struct assumes that relocations will be present in incoming wasm binary.
/// Upon construction, all the required metadata will be constructed.
pub struct Splitter<'a> {
    /// The original module we use as a reference
    source_module: Module,
    ids_to_fns: Vec<FunctionId>,
    fns_to_ids: HashMap<FunctionId, usize>,

    /// The module we're currently splitting
    output: Module,
    original: &'a [u8],
    bindgened: &'a [u8],
    split_points: Vec<SplitPoint>,
    chunks: Vec<HashSet<Node>>,
    data_symbols: BTreeMap<usize, DataSymbol>,
    main_graph: ReachabilityGraph,
    extra_graph: ReachabilityGraph,
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
    // /// The javascript that will be used to link the chunks together. Required by the wasm-bindgen
    // pub js_module: String,
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

        let _module = Module::from_buffer(&original)?;

        let mut d1 = module.data.iter();
        let mut d2 = module.data.iter();
        loop {
            let (Some(d1), Some(d2)) = (d1.next(), d2.next()) else {
                break;
            };
            tracing::debug!("d1 {:?} {:?}", d1.kind, d1.name);
            tracing::debug!("d2 {:?} {:?}", d1.kind, d1.name);
            if d1.value != d2.value {
                tracing::error!("data segments have different values");
                break;
            }
        }
        drop(d1);
        drop(d2);
        // for _ in 0..3 {

        // }

        // tracing::debug!(
        //     "There are {} data segments in the original module",
        //     _module.data.iter().count()
        // );
        // assert_eq!(module.data.iter().count(), _module.data.iter().count());

        let mut module = Self {
            source_module: module,
            original,
            bindgened,
            split_points,
            data_symbols: raw_data.data_symbols,
            ids_to_fns: ids,
            fns_to_ids,
            main_graph: Default::default(),
            chunks: Default::default(),
            call_graph: Default::default(),
            parent_graph: Default::default(),
            extra_symbols: Default::default(),
            extra_graph: Default::default(),
            output: Module::default(),
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
    pub fn emit(mut self) -> Result<OutputModules> {
        let mut modules = vec![];
        for idx in 0..self.split_points.len() {
            modules.push(self.emit_split_module(idx)?);
        }

        let mut chunks = vec![];
        // for idx in 0..self.chunks.len() {
        //     chunks.push(self.emit_split_chunk(idx)?);
        // }

        // Emit the main module, consuming self since we're going to
        let main = self.emit_main_module()?;

        Ok(OutputModules {
            modules,
            chunks,
            // js_module,
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
        // Perform some analysis of the module before we start messing with it
        let shared_funcs = self.main_shared_symbols();
        let unused_symbols = self.unused_main_symbols();

        // Use the original module that contains all the right ids
        self.output = std::mem::take(&mut self.source_module);

        // 1. Clear out the active segments that try to initialize functions for modules we just split off.
        //    When the side modules load, they will initialize functions into the table where the "holes" are.
        self.replace_segments_with_holes(&unused_symbols);

        // 2. Wipe away the unused functions and data symbols
        let deleted = self.prune_main_symbols(&unused_symbols);

        // 3. Change the functions called from split modules to be local functions that call the indirect function
        self.create_ifunc_table();

        // 4. Re-export the memories, globals, and other stuff
        self.re_export_items();

        // 5. Re-export shared functions
        self.re_export_functions(&shared_funcs);

        // 6. Remove the reloc and linking custom sections
        self.remove_custom_sections();

        // let used = Used::new(&self.output, &deleted);
        // assert!(!used.funcs.is_empty());

        // 7. Run the garbage collector to remove unused functions
        walrus::passes::gc::run(&mut self.output);

        Ok(SplitModule {
            module_name: "main".to_string(),
            component_name: None,
            bytes: self.output.emit_wasm(),
            relies_on_chunks: Default::default(),
            hash_id: None,
        })
    }

    /// Write the contents of the split modules to the output
    fn emit_split_module(&mut self, split_idx: usize) -> Result<SplitModule> {
        let split = self.split_points[split_idx].clone();

        // These are the symbols that will only exist in this module and not in the main module.
        let mut unique_symbols = split
            .reachable_graph
            .reachable
            .difference(&self.main_graph.reachable)
            .cloned()
            .collect::<HashSet<_>>();

        // The functions we'll need to import
        let mut symbols_to_import: HashSet<_> = split
            .reachable_graph
            .reachable
            .intersection(&self.main_graph.reachable)
            .cloned()
            .collect();

        // Identify the functions we'll delete
        let mut symbols_to_delete: HashSet<_> = self
            .main_graph
            .reachable
            .difference(&split.reachable_graph.reachable)
            .cloned()
            .collect();

        // for s in symbols_to_import.iter() {
        //     symbols_to_delete.remove(s);
        // }

        // for extra in self.extra_graph.reachable.iter() {
        //     // symbols_to_import.insert(extra.clone());
        //     symbols_to_delete.remove(extra);
        // }

        // let children_of_extra_symbols = self
        //     .extra_symbols
        //     .iter()
        //     .map(|s| self.call_graph.get(s).iter().copied())
        //     .collect::<HashSet<_>>();

        // // Convert split chunk functions to imports
        let mut relies_on_chunks = HashSet::new();
        // tracing::info!("There are {} chunks", self.chunks.len());
        // for (idx, chunk) in self.chunks.iter().enumerate() {
        //     for node in chunk.iter() {
        //         if self.main_graph.reachable.contains(node) {
        //             continue;
        //         }

        //         // only import this function if we actually use it in this module!
        //         if split.reachable_graph.reachable.contains(node) {
        //             unique_symbols.remove(node);
        //             symbols_to_import.insert(*node);
        //             relies_on_chunks.insert(idx);
        //         }
        //     }
        // }

        // Remap the graph to our module's IDs
        let (module, ids_to_fns, _fns_to_ids) = parse_module_with_ids(&self.bindgened)?;
        self.output = module;
        let unique_symbols = self.remap_ids(unique_symbols, &ids_to_fns);
        let symbols_to_delete = self.remap_ids(symbols_to_delete, &ids_to_fns);
        let symbols_to_import = self.remap_ids(symbols_to_import, &ids_to_fns);
        let split_export_func = ids_to_fns[self.fns_to_ids[&split.export_func]];

        // Do some basic cleanup of the module to make it smaller
        // This removes exports, imports, and the start function
        self.prune_split_module();

        // Convert tables, memories, etc to imports rather than being locally defined
        self.convert_locals_to_imports();

        // Clear away the data segments
        self.clear_data_segments(&unique_symbols);

        // Clear out the element segments and then add in the initializers for the shared imports
        self.create_ifunc_initialzers(&unique_symbols);

        // Take the symbols that are shared between the split modules and convert them to imports
        self.convert_shared_to_imports(&symbols_to_import);

        // Convert our split module's functions to real functions that call the indirect function
        self.add_split_imports(split.index, split_export_func, split.export_name);

        // Delete all the functions that are not reachable from the main module
        self.delete_main_funcs_from_split(&symbols_to_delete, &ids_to_fns);
        // let deleted = self.delete_main_funcs_from_split(&symbols_to_delete, &ids_to_fns);
        // let used_funcs = deleted
        //     .iter()
        //     .flat_map(|id| match id {
        //         Node::Function(id) => Some(*id),
        //         Node::DataSymbol(_) => None,
        //     })
        //     .collect::<HashSet<_>>();

        // let used = Used::new(&self.output, &used_funcs);

        // // Remove the reloc and linking custom sections
        self.remove_custom_sections();

        // Run the gc to remove unused functions - also validates the module to ensure we can emit it properly
        walrus::passes::gc::run(&mut self.output);

        Ok(SplitModule {
            bytes: self.output.emit_wasm(),
            module_name: split.module_name.clone(),
            component_name: Some(split.component_name.clone()),
            relies_on_chunks,
            hash_id: Some(split.hash_name.clone()),
        })
    }

    /// Write a split chunk - this is a chunk with no special functions, just exports + initializers
    fn emit_split_chunk(&mut self, idx: usize) -> Result<SplitModule> {
        let unique_symbols = self.chunks[idx].clone();

        tracing::info!("emitting chunk {}", idx);

        // Delete everything except the symbols that are reachable from this module
        let symbols_to_delete: HashSet<_> = unique_symbols
            .difference(&self.reachable_from_all())
            .cloned()
            .collect();

        // The functions we'll need to import
        let symbols_to_import: HashSet<_> = self
            .main_graph
            .reachable
            .intersection(&unique_symbols)
            .cloned()
            .collect();

        // We're going to export only the symbols that show up in other modules
        let mut symbols_to_export = HashSet::new();
        for sym in unique_symbols.iter() {
            for split in self.split_points.iter() {
                if split.reachable_graph.reachable.contains(sym) {
                    symbols_to_export.insert(*sym);
                }
            }
        }

        // Make sure to remap any ids from the main module to this module
        let (module, ids_to_fns, _fns_to_ids) = parse_module_with_ids(&self.bindgened)?;
        self.output = module;
        let unique_symbols = self.remap_ids(unique_symbols, &ids_to_fns);
        let symbols_to_export = self.remap_ids(symbols_to_export, &ids_to_fns);
        let symbols_to_import = self.remap_ids(symbols_to_import, &ids_to_fns);
        let symbols_to_delete = self.remap_ids(symbols_to_delete, &ids_to_fns);

        self.prune_split_module();

        // Convert tables, memories, etc to imports rather than being locally defined
        self.convert_locals_to_imports();

        // Clear away the data segments
        self.clear_data_segments(&unique_symbols);

        // Clear out the element segments and then add in the initializers for the shared imports
        self.create_ifunc_initialzers(&unique_symbols);

        // Take the symbols that are shared between the split modules and convert them to imports
        self.convert_shared_to_imports(&symbols_to_import);

        //
        self.re_export_functions(&symbols_to_export);

        // Make sure we haven't deleted anything important....
        let deleted = self.delete_main_funcs_from_split(&symbols_to_delete, &ids_to_fns);

        // We have to make sure our table matches that of the other tables even though we don't call them.
        let ifunc_table_id = self.load_funcref_table();
        let _segment_start = self
            .expand_ifunc_table_max(ifunc_table_id, self.split_points.len())
            .unwrap();

        // Remove the reloc and linking custom sections
        self.remove_custom_sections();

        // Run the gc to remove unused functions - also validates the module to ensure we can emit it properly
        walrus::passes::gc::run(&mut self.output);

        Ok(SplitModule {
            bytes: self.output.emit_wasm(),
            module_name: "split".to_string(),
            component_name: None,
            relies_on_chunks: Default::default(),
            hash_id: None,
        })
    }

    /// Convert any shared functions into imports
    fn convert_shared_to_imports(&mut self, symbols_to_import: &HashSet<Node>) {
        for symbol in symbols_to_import {
            if let Node::Function(id) = *symbol {
                let func = self.output.funcs.get_mut(id);
                let name = func
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("unknown - {}", id.index()));
                let ty = func.ty();
                let import =
                    self.output
                        .imports
                        .add("__wasm_split", &name, ImportKind::Function(id));
                let func = self.output.funcs.get_mut(id);
                func.kind = FunctionKind::Import(ImportedFunction { import, ty });
            }
        }
    }

    /// Convert split import functions to local functions that call an indirect function that will
    /// be filled in from the loaded split module.
    ///
    /// This is because these imports are going to be delayed until the split module is loaded
    /// and loading in the main module these as imports won't be possible since the imports won't
    /// be resolved until the split module is loaded.
    fn create_ifunc_table(&mut self) {
        let ifunc_table = self.load_funcref_table();
        let dummy_func = self.make_dummy_func();

        self.output
            .exports
            .add("__indirect_function_table", ifunc_table);

        // Expand the ifunc table to accomodate the new ifuncs
        let segment_start = self
            .expand_ifunc_table_max(ifunc_table, self.split_points.len())
            .unwrap();

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
            let ty_id = self.output.funcs.get(import_func).ty();
            let stub_idx = segment_start + ifuncs.len();

            // Replace the import function with a local function that calls the indirect function
            self.output.funcs.get_mut(import_func).kind =
                self.make_stub_funcs(ifunc_table, ty_id, stub_idx as _);

            // And remove the corresponding import
            self.output.imports.delete(import_id);

            // Push into the list the properly typed dummy func so the entry is populated
            // unclear if the typing is important here
            ifuncs.push(dummy_func);
        }

        tracing::info!("adding split imports {:?} at {}", ifuncs, segment_start);

        // Now add segments to the ifunc table
        let ifunc_table_ = self.output.tables.get_mut(ifunc_table);
        ifunc_table_.elem_segments.insert(self.output.elements.add(
            ElementKind::Active {
                table: ifunc_table,
                offset: ConstExpr::Value(ir::Value::I32(segment_start as _)),
            },
            ElementItems::Functions(ifuncs),
        ));
    }

    /// Re-export the memories, globals, and other items from the main module to the side modules
    fn re_export_items(&mut self) {
        for (idx, memory) in self.output.memories.iter().enumerate() {
            let name = memory
                .name
                .clone()
                .unwrap_or_else(|| format!("__memory_{}", idx));

            self.output.exports.add(&name, memory.id());
        }

        for (idx, global) in self.output.globals.iter().enumerate() {
            let global_name = format!("__global__{idx}");
            self.output.exports.add(&global_name, global.id());
        }

        // Export any tables
        for (idx, table) in self.output.tables.iter().enumerate() {
            if table.element_ty != RefType::Funcref {
                let table_name = format!("__imported_table_{}", idx);
                self.output.exports.add(&table_name, table.id());
            }
        }
    }

    fn re_export_functions(&mut self, shared_funcs: &HashSet<Node>) {
        // Make sure to re-export any shared functions.
        // This is somewhat in-efficient because it's re-exporting symbols that don't need to be re-exported.
        // We could just try walking the code looking for directly called functions, but that's a bit more complex.
        for func_id in shared_funcs.iter().copied() {
            if let Node::Function(func_id) = func_id {
                if self.output.exports.get_exported_func(func_id).is_none() {
                    let name = self
                        .output
                        .funcs
                        .get(func_id)
                        .name
                        .as_ref()
                        .cloned()
                        .unwrap_or_else(|| format!("unknown - {}", func_id.index()));
                    self.output.exports.add(&name, func_id);
                }
            }
        }
    }

    fn prune_main_symbols(&mut self, unused_symbols: &HashSet<Node>) -> HashSet<FunctionId> {
        // Wipe the split point exports
        for split in self.split_points.iter() {
            // it's okay that we're not re-mapping IDs since this is just used by the main module
            self.output.exports.delete(split.export_id);
        }

        let mut deleted = HashSet::new();

        // And then any actual symbols from the callgraph
        for symbol in unused_symbols.iter().cloned() {
            match symbol {
                // Simply delete functions
                Node::Function(id) => {
                    deleted.insert(id);
                    self.output.funcs.delete(id);
                }

                // Otherwise, zero out the data segment, which should lead to elimination by wasm-opt
                Node::DataSymbol(id) => {
                    // let symbols = self.data_symbols.get(&id).unwrap();
                    // tracing::info!("deleting data symbol: {:?}", symbols.name);

                    let symbol = self.data_symbols.get(&id).unwrap();
                    tracing::info!("Deleting symbol {:?}", symbol);

                    // VERY IMPORTANT
                    // apparently wasm-bindgen makes data segments that aren't the main one
                    // we definitely need to check if the
                    if symbol.which_data_segment == 0 {
                        let data_id = self.output.data.iter().next().unwrap().id();
                        let data = self.output.data.get_mut(data_id);
                        for i in symbol.segment_offset..symbol.segment_offset + symbol.symbol_size {
                            data.value[i] = 0;
                        }
                    }
                }
            }
        }

        deleted
    }

    // 2.1 Create a dummy func that will be overridden later as modules pop in
    // 2.2 swap the segment entries with the dummy func, leaving hole in its placed that will be filled in later
    fn replace_segments_with_holes(&mut self, unused_symbols: &HashSet<Node>) {
        let dummy_func = self.make_dummy_func();
        for element in self.output.elements.iter_mut() {
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

    fn create_ifunc_initialzers(&mut self, unique_symbols: &HashSet<Node>) {
        // convert shared functions to imports
        let ifunc_table = self.load_funcref_table();

        // We're going to initialize only the holes with our functions
        // eventually we can just splat the symbols in
        // since there's an empty segment at 0?
        #[derive(Clone, Copy, Hash, PartialEq, Eq)]
        enum Hole {
            Active(FunctionId),
            Passive(FunctionId, RefType),
        }

        let mut initializers = HashMap::new();
        for segment in self.output.elements.iter_mut() {
            let ElementKind::Active { offset, .. } = &mut segment.kind else {
                continue;
            };

            let ConstExpr::Value(ir::Value::I32(offset)) = offset else {
                continue;
            };

            match &segment.items {
                ElementItems::Functions(vec) => {
                    for (idx, item) in vec.into_iter().enumerate() {
                        if unique_symbols.contains(&Node::Function(*item)) {
                            initializers.insert(*offset + idx as i32, Hole::Active(*item));
                        }
                    }
                }

                ElementItems::Expressions(ref_type, const_exprs) => {
                    for (idx, expr) in const_exprs.iter().enumerate() {
                        if let ConstExpr::RefFunc(id) = expr {
                            if unique_symbols.contains(&Node::Function(*id)) {
                                initializers
                                    .insert(*offset + idx as i32, Hole::Passive(*id, *ref_type));
                            }
                        }
                    }
                }
            }
        }

        // Wipe away references to these segments
        for table in self.output.tables.iter_mut() {
            table.elem_segments.clear();
        }

        // Wipe away the segments themselves
        let segments_to_delete: Vec<_> = self.output.elements.iter().map(|e| e.id()).collect();
        for id in segments_to_delete {
            self.output.elements.delete(id);
        }

        // Add in our new segments
        let ifunc_table_ = self.output.tables.get_mut(ifunc_table);
        for (&offset, &item) in initializers.iter() {
            let kind = ElementKind::Active {
                table: ifunc_table,
                offset: ConstExpr::Value(ir::Value::I32(offset)),
            };
            let items = match item {
                Hole::Active(id) => ElementItems::Functions(vec![id]),
                Hole::Passive(id, ref_type) => {
                    ElementItems::Expressions(ref_type, vec![ConstExpr::RefFunc(id)])
                }
            };
            ifunc_table_
                .elem_segments
                .insert(self.output.elements.add(kind, items));
        }
    }

    fn add_split_imports(
        &mut self,
        split_idx: usize,
        split_export_func: FunctionId,
        split_export_name: String,
    ) {
        let ifunc_table_id = self.load_funcref_table();
        let segment_start = self
            .expand_ifunc_table_max(ifunc_table_id, self.split_points.len())
            .unwrap();

        tracing::info!(
            "segment start: {segment_start}, offset: {split_idx}, combined: {}",
            segment_start + split_idx
        );

        // Make sure to re-export the split func
        self.output
            .exports
            .add(&split_export_name, split_export_func);

        // Add the elements back to the table
        self.output
            .tables
            .get_mut(ifunc_table_id)
            .elem_segments
            .insert(self.output.elements.add(
                ElementKind::Active {
                    table: ifunc_table_id,
                    offset: ConstExpr::Value(ir::Value::I32((segment_start + split_idx) as i32)),
                },
                ElementItems::Functions(vec![split_export_func]),
            ));
    }

    fn delete_main_funcs_from_split(
        &mut self,
        symbols_to_delete: &HashSet<Node>,
        ids_to_fns: &[FunctionId],
    ) -> HashSet<Node> {
        let injected_symbols = self.remap_ids(self.extra_symbols.clone(), &ids_to_fns);
        let mut deleted_functions = HashSet::new();
        let _r = "__________".to_string();

        for node in symbols_to_delete {
            if let Node::Function(id) = *node {
                // if !injected_symbols.contains(node) {
                let func = self.output.funcs.get(id);
                let func_name = func.name.as_ref();
                let func_name = func_name.unwrap_or(&_r);

                // if func_name == "_ZN5alloc7raw_vec20RawVecInner$LT$A$GT$17try_reserve_exact17hbb1ba48adad83534E" {
                //     tracing::error!("deleting {:?}", func);
                // }

                // // we shouldn't delete unnamed functions?
                // let Some(func_name) = func_name else {
                //     tracing::error!("Could not find name for function {:?}", func);
                //     continue;
                // };

                // let FunctionKind::Local(func) = &func.kind else {
                //     continue;
                // };

                // n.contains("__externref_table_")
                // if !func_name.contains("__externref_table_") {
                self.output.funcs.delete(id);
                deleted_functions.insert(*node);
                // }
                // }
            }
        }

        deleted_functions
    }

    fn prune_split_module(&mut self) {
        // Clear the module's start/main
        if let Some(start) = self.output.start.take() {
            if let Some(export) = self.output.exports.get_exported_func(start) {
                self.output.exports.delete(export.id());
            }
        }

        // We're going to import the funcref table, so wipe it altogether
        for table in self.output.tables.iter_mut() {
            table.elem_segments.clear();
        }

        // Wipe all our imports - we're going to use a different set of imports
        let all_imports: HashSet<_> = self.output.imports.iter().map(|i| i.id()).collect();
        for import_id in all_imports {
            self.output.imports.delete(import_id);
        }

        // Wipe away all exports
        let all_exports: Vec<_> = self.output.exports.iter().map(|e| e.id()).collect();
        for export_id in all_exports {
            let export = self.output.exports.get(export_id);
            match export.item {
                ExportItem::Function(id) => {}
                ExportItem::Table(id) => {}
                ExportItem::Memory(id) => {}
                ExportItem::Global(id) => {}
            }

            self.output.exports.delete(export_id);
        }
    }

    fn make_dummy_func(&mut self) -> FunctionId {
        let mut b = FunctionBuilder::new(&mut self.output.types, &[], &[]);
        b.name("dummy".into()).func_body().unreachable();
        b.finish(vec![], &mut self.output.funcs)
    }

    fn convert_locals_to_imports(&mut self) {
        // Convert the tables to imports.
        // Should be as simple as adding a new import and then writing the `.import` field
        for (idx, table) in self.output.tables.iter_mut().enumerate() {
            let name = table.name.clone().unwrap_or_else(|| {
                if table.element_ty == RefType::Funcref {
                    format!("__indirect_function_table")
                } else {
                    format!("__imported_table_{}", idx)
                }
            });
            let import = self.output.imports.add("__wasm_split", &name, table.id());
            table.import = Some(import);
        }

        // Convert the memories to imports
        // Should be as simple as adding a new import and then writing the `.import` field
        for (idx, memory) in self.output.memories.iter_mut().enumerate() {
            let name = memory
                .name
                .clone()
                .unwrap_or_else(|| format!("__memory_{}", idx));
            let import = self.output.imports.add("__wasm_split", &name, memory.id());
            memory.import = Some(import);
        }

        // Convert the globals to imports
        let global_ids: Vec<_> = self.output.globals.iter().map(|t| t.id()).collect();
        for (idx, global_id) in global_ids.into_iter().enumerate() {
            let global = self.output.globals.get_mut(global_id);
            let global_name = format!("__global__{idx}");
            let import = self
                .output
                .imports
                .add("__wasm_split", &global_name, global.id());
            global.kind = GlobalKind::Import(import);
        }
    }

    fn clear_data_segments(&mut self, unique_symbols: &HashSet<Node>) {
        // Preserve the data symbols for this module and then clear them away
        let data_ids: Vec<_> = self.output.data.iter().map(|t| t.id()).collect();
        for (idx, data_id) in data_ids.into_iter().enumerate() {
            let data = self.output.data.get_mut(data_id);

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
                    let range = symbol.segment_offset..symbol.segment_offset + symbol.symbol_size;
                    let offset = ConstExpr::Value(ir::Value::I32(
                        data_offset + symbol.segment_offset as i32,
                    ));
                    self.output.data.add(
                        DataKind::Active { memory, offset },
                        contents[range].to_vec(),
                    );
                }
            }
        }
    }

    /// Load the funcref table from the main module. This *should* exist for all modules created by
    /// Rustc or Wasm-Bindgen, but we create it if it doesn't exist.
    fn load_funcref_table(&mut self) -> TableId {
        let ifunc_table = self
            .output
            .tables
            .iter()
            .find(|t| t.element_ty == RefType::Funcref)
            .map(|t| t.id());

        if let Some(table) = ifunc_table {
            table
        } else {
            self.output
                .tables
                .add_local(false, 0, None, RefType::Funcref)
        }
    }

    /// Convert the imported function to a local function that calls an indirect function from the table
    ///
    /// This will enable the main module (and split modules) to call functions from outside their own module.
    /// The functions might not exist when the main module is loaded, so we'll register some elements
    /// that fill those in eventually.
    fn make_stub_funcs(&mut self, table: TableId, ty_id: TypeId, table_idx: i32) -> FunctionKind {
        // Convert the import function to a local function that calls the indirect function from the table
        let ty = self.output.types.get(ty_id);

        let params = ty.params().to_vec();
        let results = ty.results().to_vec();
        let args: Vec<_> = params
            .iter()
            .map(|ty| self.output.locals.add(*ty))
            .collect();

        // New function that calls the indirect function
        let mut builder = FunctionBuilder::new(&mut self.output.types, &params, &results);
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
    fn expand_ifunc_table_max(&mut self, table: TableId, num_ifuncs: usize) -> Option<usize> {
        let ifunc_table_ = self.output.tables.get_mut(table);

        if let Some(max) = ifunc_table_.maximum {
            ifunc_table_.maximum = Some(max + num_ifuncs as u64);
            ifunc_table_.initial += num_ifuncs as u64;
            return Some(max as usize);
        }

        None
    }

    fn remove_custom_sections(&mut self) {
        let sections_to_delete = self
            .output
            .customs
            .iter()
            .filter_map(|(id, section)| {
                if section.name().contains("linking") || section.name().contains("reloc") {
                    Some(id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for id in sections_to_delete {
            self.output.customs.delete(id);
        }
    }

    /// Use the Louvain algorithm (okay not actually, is just greedy right now)
    ///  to determine communties in the split modules so we can create  efficient chunks
    fn build_split_chunks(&mut self) {
        // Every symbol and the chunks that use it
        // We're only going to try optimizing functions used across multiple chunks
        let mut funcs_used_by_chunks: HashMap<Node, HashSet<usize>> = HashMap::new();
        for split in self.split_points.iter() {
            for item in split.reachable_graph.reachable.iter() {
                funcs_used_by_chunks
                    .entry(item.clone())
                    .or_default()
                    .insert(split.index);
            }
        }

        // Remove all the chunks that are only used by one module
        funcs_used_by_chunks.retain(|_, v| v.len() > 1);

        const MAX_CHUNK_SIZE: usize = 100000;
        let mut remaining_functions: BTreeSet<Node> =
            funcs_used_by_chunks.keys().cloned().collect();

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
            shared_funcs.extend(
                split
                    .reachable_graph
                    .reachable
                    .intersection(&self.main_graph.reachable),
            );
        }

        for injected in self.extra_symbols.iter() {
            shared_funcs.insert(*injected);
        }

        shared_funcs
    }

    fn unused_main_symbols(&self) -> HashSet<Node> {
        let mut unique = HashSet::new(); // self.main_graph.reachable.clone();

        for split in self.split_points.iter() {
            let roots = Some(Node::Function(split.export_func))
                .into_iter()
                .collect::<HashSet<Node>>();

            let graph = ReachabilityGraph::new(&self.call_graph, &roots, &Default::default());

            let mut unique_symbols = graph
                .reachable
                .difference(&self.main_graph.reachable)
                .cloned()
                .collect::<HashSet<_>>();

            unique.extend(unique_symbols);
        }

        let mut fix_expots = vec![];
        for _u in unique.iter() {
            if self.extra_symbols.contains(_u) {
                tracing::error!("found extra symbol: {:?}", _u);
            }

            if self.main_graph.reachable.contains(_u) {
                tracing::error!("found main symbol: {:?}", _u);
            }

            if let Node::Function(_u) = _u {
                if self.source_module.exports.get_exported_func(*_u).is_some() {
                    tracing::error!("found exported symbol: {:?}", _u);
                    fix_expots.push(*_u);
                }
            }
        }

        for _u in fix_expots.iter() {
            tracing::warn!("fixing export: {:?}", _u);
            unique.remove(&Node::Function(*_u));
        }

        // let mut funcs_split_away = HashSet::new();
        // Collect *every* symbol
        // let all = self.reachable_from_all();

        // all.difference()
        //     .cloned()
        //     .collect()

        // // // get the reachable symbols from every split combined with main
        // // let mut reachable_from_every = self.main_graph.reachable.clone();
        // for split in self.split_points.iter() {
        //     // reachable_from_every.extend(split_reachable.reachable.iter().cloned());
        //     unique.extend(
        //         (&split.reachable_graph)
        //             .reachable
        //             .difference(&self.main_graph.reachable),
        //     );
        // }

        // for import in self.source_module.imports.iter() {
        //     if let ImportKind::Function(func) = import.kind {
        //         unique.remove(&Node::Function(func));
        //     }
        // }

        // for export in self.source_module.exports.iter() {
        //     if let ExportItem::Function(func) = export.item {
        //         unique.remove(&Node::Function(func));
        //     }
        // }

        // // These are symbols we can't delete in the main module
        // // let to_save: HashSet<Node> = all.difference(&reachable_from_every).cloned().collect();
        // // unique.difference(&to_save).cloned().collect()
        unique
    }

    fn reachable_from_all(&self) -> HashSet<Node> {
        let mut reachable = HashSet::new();
        for (key, f) in self.call_graph.iter() {
            reachable.insert(*key);
            reachable.extend(f.into_iter());
        }
        reachable
    }

    /// Accumulate the relocations from the original module, create a relocation map, and then convert
    /// that to our *new* module's symbols.
    fn build_call_graph(&mut self) -> Result<()> {
        let original = ModuleWithRelocations::new(&self.original)?;

        let new_funcs: HashMap<String, FunctionId> = self
            .source_module
            .funcs
            .iter()
            .enumerate()
            .map(|(idx, f)| {
                (
                    f.name
                        .as_ref()
                        .cloned()
                        .unwrap_or_else(|| format!("__unknown_{idx}")),
                    f.id(),
                )
            })
            .collect();
        let new_data: HashMap<&String, &DataSymbol> = self
            .data_symbols
            .iter()
            .map(|(_, s)| (&s.name, s))
            .collect();

        let get_func = |old_id: FunctionId| {
            let name = original.module.funcs.get(old_id).name.as_ref()?;
            new_funcs.get(name).map(|id| Node::Function(*id))
        };

        let get_data = |id: usize| {
            let symbol = original.data_symbols.get(&id)?;
            let symbol = new_data.get(&symbol.name)?;
            Some(Node::DataSymbol(symbol.index))
        };

        for (key, value) in original.call_graph.iter() {
            let children = value
                .iter()
                .flat_map(|node| match node {
                    Node::Function(id) => get_func(*id),
                    Node::DataSymbol(id) => get_data(*id),
                })
                .collect::<HashSet<_>>();

            let entry = match key {
                Node::Function(id) => get_func(*id),
                Node::DataSymbol(id) => get_data(*id),
            };

            if let Some(node) = entry {
                self.call_graph.insert(node, children);
            } else {
                for child in children.iter() {
                    let _p = self.call_graph.entry(*child).or_default();
                }

                self.extra_symbols.extend(children.into_iter())
            }
        }

        // Build the parent graph
        for (func, children) in &self.call_graph {
            for child in children {
                self.parent_graph.entry(*child).or_default().insert(*func);
            }
        }

        // Now go fill in the reachability graph for each of the split points
        self.split_points.iter_mut().for_each(|split| {
            let mut roots: HashSet<_> = Some(Node::Function(split.export_func))
                .into_iter()
                .collect();

            for export in self.source_module.exports.iter() {
                if let ExportItem::Function(id) = export.item {
                    if export.name.contains("__wasm_split") || export.name.contains("main") {
                        continue;
                    }
                    roots.insert(Node::Function(id));
                }
            }

            for import in self.source_module.imports.iter() {
                if let ImportKind::Function(id) = import.kind {
                    if import.name.contains("__wasm_split") || import.name.contains("main") {
                        continue;
                    }
                    roots.insert(Node::Function(id));
                }
            }

            split.reachable_graph =
                ReachabilityGraph::new(&self.call_graph, &roots, &Default::default());
        });

        // And then the reachability graph for main
        self.main_graph =
            ReachabilityGraph::new(&self.call_graph, &self.main_roots(), &Default::default());

        // // If there's no dep counted for by the split reachable graphs, insert it into the main reachable graph
        // self.main_graph
        //     .reachable
        //     .extend(self.extra_symbols.iter().cloned());

        // let mut reachable_from_extra = self.extra_symbols.clone();

        // let reachable_from_extra =
        //     ReachabilityGraph::new(&self.call_graph, &self.extra_symbols, &Default::default());

        // for export in self.source_module.exports.iter() {
        //     tracing::error!("export: {:?}", export);
        // }

        // // tracing::error!(
        // //     "reachable from extra: {:#?}",
        // //     reachable_from_extra.reachable
        // // );

        // for item in reachable_from_extra.reachable.iter() {
        //     if let Node::Function(id) = *item {
        //         let func = self.source_module.funcs.get(id);
        //         let name = func.name.as_ref().unwrap();
        //         tracing::error!("reachable from extra: {:?}", name);
        //     }
        //     self.main_graph.reachable.insert(*item);
        // }

        // self.extra_graph = reachable_from_extra;

        // for split in self.split_points.iter_mut() {
        //     split
        //         .reachable_graph
        //         .reachable
        //         .extend(self.extra_symbols.iter().cloned());
        // }

        // let name =
        //     "_ZN5alloc7raw_vec20RawVecInner;$LT$A$GT$17try_reserve_exact17hbb1ba48adad83534E";
        // let name = "__externref_table_alloc";
        // for export in self.source_module.exports.iter() {
        //     if export.name == name {
        //         println!("found it as an export: {:?}", export);
        //     }
        // }

        // for import in self.source_module.imports.iter() {
        //     if import.name == name {
        //         println!("found it as an import: {:?}", import);
        //     }
        // }

        // for global in self.source_module.globals.iter() {
        //     if global.name.as_deref() == Some(name) {
        //         println!("found it as a global: {:?}", global);
        //     }
        // }

        // let func = self.source_module.funcs.by_name(name).unwrap();
        // let node = Node::Function(func);
        // tracing::error!("extra func: {:?}", func);
        // let mut parents = self.parent_graph.get(&node).unwrap().clone();
        // loop {
        //     let Some(parent) = parents.iter().cloned().next() else {
        //         break;
        //     };
        //     parents.remove(&parent);
        //     if let Node::Function(parent) = parent {
        //         let func = self.source_module.funcs.get(parent);
        //         let name = func.name.as_ref().unwrap();
        //         tracing::error!("parent func: {:?}", name);
        //         if name == "__externref_table_alloc" {
        //             self.main_graph.reachable.insert(Node::Function(parent));

        //             for split in self.split_points.iter_mut() {
        //                 split
        //                     .reachable_graph
        //                     .reachable
        //                     .insert(Node::Function(parent));
        //             }
        //         }
        //     }
        // }
        // if self.extra_graph.reachable.contains(&node) {
        //     tracing::error!("found it in the extra graph");
        // }

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

        let imported_splits = self
            .split_points
            .iter()
            .map(|f| f.import_func)
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

        // Also add "imports" to the roots
        for import in self.source_module.imports.iter() {
            if let ImportKind::Function(id) = import.kind {
                // if !imported_splits.contains(&id) {
                roots.insert(Node::Function(id));
                // }
            }
        }

        for export in self.source_module.exports.iter() {
            if export.name.contains("__wasm_split_00") {
                tracing::debug!("Skipping split export: {:?}", export.name);
                continue;
            }

            if let ExportItem::Function(id) = export.item {
                roots.insert(Node::Function(id));
            }
        }

        for extra in self.extra_symbols.iter() {
            roots.insert(*extra);
        }

        roots
    }

    /// Convert this set of nodes to reference the new module
    fn remap_ids(&self, set: HashSet<Node>, ids_to_fns: &[FunctionId]) -> HashSet<Node> {
        let mut out = HashSet::with_capacity(set.len());

        for node in set {
            match node {
                // Remap the function IDs
                Node::Function(id) => out.insert(Node::Function(ids_to_fns[self.fns_to_ids[&id]])),
                // data symbols don't need remapping
                Node::DataSymbol(id) => out.insert(Node::DataSymbol(id)),
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
            let mut ids = ids_.write().unwrap();
            let mut idx = 0;
            while let Ok(entry) = our_ids.get_func(idx) {
                ids.push(entry);
                idx += 1;
            }

            Ok(())
        }),
    )?;
    let mut ids_ = ids.write().unwrap();
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
    call_graph: HashMap<Node, HashSet<Node>>,
    relocation_map: HashMap<Node, Vec<RelocationEntry>>,
    data_symbols: BTreeMap<usize, DataSymbol>,
    data_section_range: Range<usize>,
}

impl<'a> ModuleWithRelocations<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self> {
        let module = Module::from_buffer(bytes)?;
        let raw_data = parse_bytes_to_data_segment(&bytes)?;

        let mut module = Self {
            module,
            data_symbols: raw_data.data_symbols,
            data_section_range: raw_data.data_range,
            symbols: raw_data.symbols,
            call_graph: Default::default(),
            relocation_map: Default::default(),
        };

        module.build_code_call_graph()?;
        module.build_data_call_graph()?;

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

            while let Some(entry) =
                relocations.next_if(|entry| entry.relocation_range().start < range.end)
            {
                let reloc_range = entry.relocation_range();
                assert!(reloc_range.start >= range.start);
                assert!(reloc_range.end <= range.end);

                if let Some(target) = self.get_symbol_dep_node(entry.index as usize) {
                    let dep = Node::Function(func_id);
                    self.call_graph.entry(dep).or_default().insert(target);
                    self.relocation_map.entry(dep).or_default().push(*entry);
                } else {
                    // tracing::error!("No target of relocation {:?}", entry);
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

                if let Some(target) = self.get_symbol_dep_node(entry.index as usize) {
                    let dep = Node::DataSymbol(symbol.index);
                    self.call_graph.entry(dep).or_default().insert(target);
                    self.relocation_map.entry(dep).or_default().push(*entry);
                } else {
                    // tracing::error!("No target of data relocation {:?}", entry);
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
            .unwrap();

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
    fn get_symbol_dep_node(&self, index: usize) -> Option<Node> {
        match self.symbols[index] {
            SymbolInfo::Data { .. } => Some(Node::DataSymbol(index)),
            SymbolInfo::Func { name, .. } => Some(Node::Function(
                self.module
                    .funcs
                    .by_name(name.expect("local func symbol without name?"))
                    .unwrap(), // .unwrap_or_else(|| panic!("local func symbol without name: {name:?}")),
            )),
            SymbolInfo::Global { flags, index, name } => {
                // tracing::error!("Global symbol: {:?} {:?} {:?}", flags, index, name);
                None
            }
            SymbolInfo::Event { flags, index, name } => {
                // tracing::error!("Event symbol: {:?} {:?} {:?}", flags, index, name);
                None
            }

            _ => None,
        }
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
    reachable_graph: ReachabilityGraph,
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

#[derive(Debug, Default, Clone)]
pub struct ReachabilityGraph {
    reachable: HashSet<Node>,
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, PartialOrd, Ord, Clone)]
pub enum Node {
    Function(FunctionId),
    DataSymbol(usize),
}

impl ReachabilityGraph {
    fn new(
        deps: &HashMap<Node, HashSet<Node>>,
        roots: &HashSet<Node>,
        exclude: &HashSet<Node>,
    ) -> ReachabilityGraph {
        let mut queue: VecDeque<Node> = roots.iter().copied().collect();
        let mut reachable = HashSet::<Node>::new();
        let mut parents = HashMap::<Node, Node>::new();

        while let Some(node) = queue.pop_front() {
            reachable.insert(node);
            let Some(children) = deps.get(&node) else {
                continue;
            };
            for child in children {
                if reachable.contains(&child) || exclude.contains(&child) {
                    continue;
                }
                parents.entry(*child).or_insert(node);
                queue.push_back(*child);
            }
        }

        ReachabilityGraph { reachable }
    }
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
                segments = section.into_iter().collect::<Result<Vec<_>, _>>().unwrap();
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

        let data_segment = segments.get(symbol.index as usize).unwrap();
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

// name: "__data_end", item: Global(Id { idx: 1 }) }
// name: "__heap_base", item: Global(Id { idx: 2 }) }
// name: "__wbindgen_malloc", item: Function(Id { idx: 7188 }) }
// name: "__wbindgen_realloc", item: Function(Id { idx: 8506 }) }
// name: "__wbindgen_export_2", item: Table(Id { idx: 1 }) }
// name: "__wbindgen_exn_store", item: Function(Id { idx: 11195 }) }
// name: "__externref_table_alloc", item: Function(Id { idx: 2266 }) }
// name: "__wbindgen_free", item: Function(Id { idx: 11194 }) }
// name: "__externref_drop_slice", item: Function(Id { idx: 8742 }) }
// name: "__wbindgen_export_7", item: Table(Id { idx: 0 }) }
// name: "closure660_externref_shim", item: Function(Id { idx: 11202 }) }
// name: "_ZN132__LT_dyn_u20_core__ops__function__FnMut_LT__LP__RP__GT__u2b_Output_u20__u3d__u20_R_u20_as_u20_wasm_bindgen__closure__WasmClosure_GT_8describe6invoke17h0a0f8b813b3dbe51E", item: Function(Id { idx: 9592 }) }
// name: "_ZN133__LT_dyn_u20_core__ops__function__Fn_LT__LP_A_C__RP__GT__u2b_Output_u20__u3d__u20_R_u20_as_u20_wasm_bindgen__closure__WasmClosure_GT_8describe6invoke17h98fe2c2f4a90478bE_multivalue_shim", item: Function(Id { idx: 8897 }) }
// name: "_ZN136__LT_dyn_u20_core__ops__function__FnMut_LT__LP_A_C__RP__GT__u2b_Output_u20__u3d__u20_R_u20_as_u20_wasm_bindgen__closure__WasmClosure_GT_8describe6invoke17h536c21008fca9f75E", item: Function(Id { idx: 5692 }) }
// name: "closure677_externref_shim", item: Function(Id { idx: 8896 }) }
// name: "__wbindgen_start", item: Function(Id { idx: 13629 }) }

//  root: "main"
//  root: "__wbg_jsowner_free"
//  root: "__wbindgen_malloc"
//  root: "__wbindgen_realloc"
//  root: "__wbindgen_exn_store"
//  root: "__externref_table_alloc"
//  root: "__wbindgen_free"
//  root: "__externref_drop_slice"
//  root: "closure660_externref_shim"
//  root: "_ZN132__LT_dyn_u20_core__ops__function__FnMut_LT__LP__RP__GT__u2b_Output_u20__u3d__u20_R_u20_as_u20_wasm_bindgen__closure__WasmClosure_GT_8describe6invoke17h0a0f8b813b3dbe51E"
//  root: "_ZN133__LT_dyn_u20_core__ops__function__Fn_LT__LP_A_C__RP__GT__u2b_Output_u20__u3d__u20_R_u20_as_u20_wasm_bindgen__closure__WasmClosure_GT_8describe6invoke17h98fe2c2f4a90478bE_multivalue_shim"
//  root: "_ZN136__LT_dyn_u20_core__ops__function__FnMut_LT__LP_A_C__RP__GT__u2b_Output_u20__u3d__u20_R_u20_as_u20_wasm_bindgen__closure__WasmClosure_GT_8describe6invoke17h536c21008fca9f75E"
//  root: "closure677_externref_shim"
//  root: "__wbindgen_start"

#[test]
fn test_split() {
    let original = include_bytes!("../data/dioxus_docs_site.wasm");
    let bindgen = include_bytes!("../data/bindgen/main_bg.wasm");

    // let mut original = Module::from_buffer(original).unwrap();
    // let mut bindgen = Module::from_buffer(bindgen).unwrap();

    // // for export in module.exports.iter() {
    // //     println!("{:?}", export);
    // // }
    // let mut exports_to_delete = vec![];
    // for export in bindgen.exports.iter() {
    //     if export.name.contains("__wasm_split_00") {
    //         exports_to_delete.push(export.id());
    //     } else {
    //         println!("{:?}", export);
    //     }
    // }

    // for export in exports_to_delete {
    //     bindgen.exports.delete(export);
    // }

    let mut splitter = Splitter::new(original, bindgen).unwrap();
    splitter.emit().unwrap();
}
