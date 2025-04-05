use std::collections::HashSet;

use id_arena::Id;
use walrus::{ir::*, ExportId};
use walrus::{ConstExpr, Data, DataId, DataKind, Element, ExportItem, Function};
use walrus::{ElementId, ElementItems, ElementKind, Module, RefType, Type, TypeId};
use walrus::{FunctionId, FunctionKind, Global, GlobalId};
use walrus::{GlobalKind, Memory, MemoryId, Table, TableId};

type IdHashSet<T> = HashSet<Id<T>>;

/// Set of all root used items in a wasm module.
#[derive(Debug, Default)]
pub struct Roots {
    tables: Vec<TableId>,
    funcs: Vec<(FunctionId, Location)>,
    globals: Vec<GlobalId>,
    memories: Vec<MemoryId>,
    data: Vec<DataId>,
    elements: Vec<ElementId>,
    used: Used,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Location {
    Start,
    Export { export: ExportId },
    Table { table: TableId },
    Memory { memory: MemoryId },
    Global { global: GlobalId },
    Data,
    Element { element: ElementId },
    Code { func: FunctionId },
}

impl Roots {
    /// Creates a new set of empty roots.
    pub fn new() -> Roots {
        Roots::default()
    }

    /// Adds a new function to the set of roots
    pub fn push_func(&mut self, func: FunctionId, from: Location) -> &mut Roots {
        if self.used.funcs.insert(func) {
            // log::trace!("function is used: {:?}", func);
            self.funcs.push((func, from));
        }
        self
    }

    /// Adds a new table to the set of roots
    pub fn push_table(&mut self, table: TableId) -> &mut Roots {
        if self.used.tables.insert(table) {
            // log::trace!("table is used: {:?}", table);
            self.tables.push(table);
        }
        self
    }

    /// Adds a new memory to the set of roots
    pub fn push_memory(&mut self, memory: MemoryId) -> &mut Roots {
        if self.used.memories.insert(memory) {
            // log::trace!("memory is used: {:?}", memory);
            self.memories.push(memory);
        }
        self
    }

    /// Adds a new global to the set of roots
    pub fn push_global(&mut self, global: GlobalId) -> &mut Roots {
        if self.used.globals.insert(global) {
            // log::trace!("global is used: {:?}", global);
            self.globals.push(global);
        }
        self
    }

    fn push_data(&mut self, data: DataId) -> &mut Roots {
        if self.used.data.insert(data) {
            // log::trace!("data is used: {:?}", data);
            self.data.push(data);
        }
        self
    }

    fn push_element(&mut self, element: ElementId) -> &mut Roots {
        if self.used.elements.insert(element) {
            // log::trace!("element is used: {:?}", element);
            self.elements.push(element);
        }
        self
    }
}

/// Finds the things within a module that are used.
///
/// This is useful for implementing something like a linker's `--gc-sections` so
/// that our emitted `.wasm` binaries are small and don't contain things that
/// are not used.
#[derive(Debug, Default)]
pub struct Used {
    /// The module's used tables.
    pub tables: IdHashSet<Table>,
    /// The module's used types.
    pub types: IdHashSet<Type>,
    /// The module's used functions.
    pub funcs: IdHashSet<Function>,
    /// The module's used globals.
    pub globals: IdHashSet<Global>,
    /// The module's used memories.
    pub memories: IdHashSet<Memory>,
    /// The module's used passive element segments.
    pub elements: IdHashSet<Element>,
    /// The module's used passive data segments.
    pub data: IdHashSet<Data>,
}

impl Used {
    /// Construct a new `Used` set for the given module.
    pub fn new(module: &Module, deleted: &HashSet<FunctionId>) -> Used {
        // log::debug!("starting to calculate used set");
        let mut stack = Roots::default();

        // All exports are roots
        for export in module.exports.iter() {
            match export.item {
                ExportItem::Function(f) => stack.push_func(
                    f,
                    Location::Export {
                        export: export.id(),
                    },
                ),
                ExportItem::Table(t) => stack.push_table(t),
                ExportItem::Memory(m) => stack.push_memory(m),
                ExportItem::Global(g) => stack.push_global(g),
            };
        }

        // The start function is an implicit root as well
        if let Some(f) = module.start {
            stack.push_func(f, Location::Start);
        }

        // Initialization of memories or tables is a side-effectful operation
        // because they can be out-of-bounds, so keep all active segments.
        for data in module.data.iter() {
            if let DataKind::Active { .. } = &data.kind {
                stack.push_data(data.id());
            }
        }
        for elem in module.elements.iter() {
            match elem.kind {
                // Active segments are rooted because they initialize imported
                // tables.
                ElementKind::Active { table, .. } => {
                    if module.tables.get(table).import.is_some() {
                        stack.push_element(elem.id());
                    }
                }
                // Declared segments can probably get gc'd but for now we're
                // conservative and we root them
                ElementKind::Declared => {
                    stack.push_element(elem.id());
                }
                ElementKind::Passive => {}
            }
        }

        // // And finally ask custom sections for their roots
        // for (_id, section) in module.customs.iter() {
        //     section.add_gc_roots(&mut stack);
        // }
        // tracing::info!("Used roots: {:#?}", stack);

        // Iteratively visit all items until our stack is empty
        while !stack.funcs.is_empty()
            || !stack.tables.is_empty()
            || !stack.memories.is_empty()
            || !stack.globals.is_empty()
            || !stack.data.is_empty()
            || !stack.elements.is_empty()
        {
            while let Some((f, _loc)) = stack.funcs.pop() {
                if deleted.contains(&f) {
                    let func = module.funcs.get(f);
                    let name = func
                        .name
                        .as_ref()
                        .cloned()
                        .unwrap_or_else(|| format!("unknown - {}", f.index()));
                    // panic!(
                    //     "Found a function that should be deleted but is still used: {:?} - {:?}",
                    //     name, f
                    // );
                    tracing::error!(
                        "Found a function that should be deleted but is still used: {:?} - {:?} - {:?}",
                        name,
                        f,
                        _loc
                    );
                    if let Location::Code { func } = _loc {
                        let func_name = module.funcs.get(func).name.as_ref().unwrap();
                        tracing::error!("Function {:?} is used by {:?}", f, func_name);
                    }

                    // continue;
                }

                let func = module.funcs.get(f);
                stack.used.types.insert(func.ty());

                match &func.kind {
                    FunctionKind::Local(func) => {
                        let mut visitor = UsedVisitor {
                            cur_func: f,
                            stack: &mut stack,
                        };
                        dfs_in_order(&mut visitor, func, func.entry_block());
                    }
                    FunctionKind::Import(_) => {}
                    FunctionKind::Uninitialized(_) => unreachable!(),
                }
            }

            while let Some(t) = stack.tables.pop() {
                for elem in module.tables.get(t).elem_segments.iter() {
                    stack.push_element(*elem);
                }
            }

            while let Some(t) = stack.globals.pop() {
                match &module.globals.get(t).kind {
                    GlobalKind::Import(_) => {}
                    GlobalKind::Local(ConstExpr::Global(global)) => {
                        stack.push_global(*global);
                    }
                    GlobalKind::Local(ConstExpr::RefFunc(func)) => {
                        stack.push_func(*func, Location::Global { global: t });
                    }
                    GlobalKind::Local(ConstExpr::Value(_))
                    | GlobalKind::Local(ConstExpr::RefNull(_)) => {}
                }
            }

            while let Some(t) = stack.memories.pop() {
                for data in &module.memories.get(t).data_segments {
                    stack.push_data(*data);
                }
            }

            while let Some(d) = stack.data.pop() {
                let d = module.data.get(d);
                if let DataKind::Active { memory, offset } = &d.kind {
                    stack.push_memory(*memory);
                    if let ConstExpr::Global(g) = offset {
                        stack.push_global(*g);
                    }
                }
            }

            while let Some(e) = stack.elements.pop() {
                let e = module.elements.get(e);
                if let ElementItems::Functions(function_ids) = &e.items {
                    function_ids.iter().for_each(|f| {
                        stack.push_func(*f, Location::Element { element: e.id() });
                    });
                }
                if let ElementItems::Expressions(RefType::Funcref, items) = &e.items {
                    for item in items {
                        match item {
                            ConstExpr::Global(g) => {
                                stack.push_global(*g);
                            }
                            ConstExpr::RefFunc(f) => {
                                stack.push_func(*f, Location::Element { element: e.id() });
                            }
                            _ => {}
                        }
                    }
                }
                if let ElementKind::Active { offset, table } = &e.kind {
                    if let ConstExpr::Global(g) = offset {
                        stack.push_global(*g);
                    }
                    stack.push_table(*table);
                }
            }
        }

        // Wabt seems to have weird behavior where a `data` segment, if present
        // even if passive, requires a `memory` declaration. Our GC pass is
        // pretty aggressive and if you have a passive data segment and only
        // `data.drop` instructions you technically don't need the `memory`.
        // Let's keep `wabt` passing though and just say that if there are data
        // segments kept, but no memories, then we try to add the first memory,
        // if any, to the used set.
        if !stack.used.data.is_empty() && stack.used.memories.is_empty() {
            if let Some(mem) = module.memories.iter().next() {
                stack.used.memories.insert(mem.id());
            }
        }

        stack.used
    }
}

struct UsedVisitor<'a> {
    cur_func: FunctionId,
    stack: &'a mut Roots,
}

impl Visitor<'_> for UsedVisitor<'_> {
    fn visit_function_id(&mut self, &func: &FunctionId) {
        self.stack.push_func(
            func,
            Location::Code {
                func: self.cur_func,
            },
        );
    }

    fn visit_memory_id(&mut self, &m: &MemoryId) {
        self.stack.push_memory(m);
    }

    fn visit_global_id(&mut self, &g: &GlobalId) {
        self.stack.push_global(g);
    }

    fn visit_table_id(&mut self, &t: &TableId) {
        self.stack.push_table(t);
    }

    fn visit_type_id(&mut self, &t: &TypeId) {
        self.stack.used.types.insert(t);
    }

    fn visit_data_id(&mut self, &d: &DataId) {
        self.stack.push_data(d);
    }

    fn visit_element_id(&mut self, &e: &ElementId) {
        self.stack.push_element(e);
    }
}
