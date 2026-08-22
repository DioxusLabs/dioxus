//! Ad hoc wasm inspection tool for debugging `wasm-split-cli` output -
//! table slots, relocations, and data-segment/symbol-table layout. Not
//! wired into the real `dx`/wasm-split-cli build - `cargo run --release
//! --bin diag -- <path.wasm> [name-substring]`, or with one of:
//!   DUMP_INSTRS=1        - also dump full instruction lists for matches
//!   FIND_CONST=<n>       - list every local function whose entry block
//!                          contains the i32/i64 constant `n`
//!   FIND_TABLE_SLOT=<s>  - find what ifunc-table slot a name-matched
//!                          function occupies (needs real names, i.e. a
//!                          pre-split / `--debug-symbols` build)
//!   WHAT_IS_AT_SLOT=<n>  - dump whatever function occupies absolute table
//!                          slot `n` (works even with names stripped)
//!   RANGE_OF=<name>      - print a name-matched function's original_range
//!   VALIDATE_RELOC=<name> - dump reloc.CODE entries for name-matched
//!                          functions (note: `loc` in the instruction dump
//!                          is NOT directly comparable to a relocation's
//!                          `offset` - they're in different coordinate
//!                          systems, offset by the code section's start;
//!                          subtract the first instruction's own `loc` from
//!                          a later one to get a position comparable to
//!                          `offset - range.start`)
//!   DATA_AT=<addr>        - dump raw bytes at a virtual address, reading
//!                          across every active data segment as one flat
//!                          memory image (DATA_LEN=<n> for length, default
//!                          64) - a pointer's pointee can straddle two
//!                          segments, so this never clips at one segment's
//!                          own boundary
//!   DATA_SEGMENTS=1       - list every data segment (active/passive) and
//!                          any memory.init/data.drop bulk-memory ops
//!   SYMBOLS_AT=<addr>     - dump linking-section symbol table entries
//!                          (name, declared size, real virtual address
//!                          range) near a given address - the tool for
//!                          finding overlapping/tail-merged data symbols
//!   FUNC_ID=<n>           - full instruction dump for the function whose
//!                          walrus FunctionId index is exactly `n`
use std::collections::HashSet;
use std::env;

use walrus::{
    FunctionId, FunctionKind, Module,
    ir::{self, Visitor, dfs_in_order},
};

fn main() {
    let path = env::args().nth(1).expect("wasm path arg 1");
    let filter = env::args().nth(2).unwrap_or_default();

    let module = Module::from_file(&path).expect("parse module");

    if let Ok(addr_str) = env::var("SYMBOLS_AT") {
        use wasmparser::{BinaryReader, Linking, LinkingSectionReader, Payload, SymbolInfo};
        let addr: u32 = addr_str.parse().unwrap();

        // Real virtual memory address of each data segment, in file order,
        // straight from walrus (which decodes the active-offset init expr
        // for us - wasmparser's raw Data only gives file-relative ranges).
        let seg_active_offsets: Vec<Option<i32>> = module
            .data
            .iter()
            .map(|d| match &d.kind {
                walrus::DataKind::Active {
                    offset: walrus::ConstExpr::Value(walrus::ir::Value::I32(off)),
                    ..
                } => Some(*off),
                _ => None,
            })
            .collect();

        let bytes = std::fs::read(&path).expect("read wasm file");
        let parser = wasmparser::Parser::new(0);
        let mut symbols = vec![];
        for payload in parser.parse_all(&bytes) {
            if let Payload::CustomSection(section) = payload.expect("parse payload") {
                if section.name() == "linking" {
                    let reader = BinaryReader::new(section.data(), 0);
                    let reader = LinkingSectionReader::new(reader).expect("parse linking section");
                    for subsection in reader.subsections() {
                        if let Linking::SymbolTable(map) = subsection.expect("parse subsection") {
                            symbols = map
                                .into_iter()
                                .collect::<Result<Vec<_>, _>>()
                                .expect("parse symtab");
                        }
                    }
                }
            }
        }
        println!("total symtab entries: {}", symbols.len());
        println!("data segments (virtual addr): {:?}", seg_active_offsets);
        for (index, symbol) in symbols.iter().enumerate() {
            let SymbolInfo::Data {
                name,
                symbol: Some(sym),
                ..
            } = symbol
            else {
                continue;
            };
            let Some(Some(seg_addr)) = seg_active_offsets.get(sym.index as usize) else {
                continue;
            };
            let true_start = *seg_addr as usize + sym.offset as usize;
            let true_end = true_start + sym.size as usize;
            let addr = addr as usize;
            if true_start.saturating_sub(32) <= addr && addr <= true_end + 32 {
                println!(
                    "symtab[{index}] name={name:?} data_segment_idx={} segment_offset={} size={} => addr_range=[{true_start}, {true_end}) contains_target={}",
                    sym.index,
                    sym.offset,
                    sym.size,
                    addr >= true_start && addr < true_end
                );
            }
        }
        return;
    }

    println!("module funcs: {}", module.funcs.iter().count());

    if let Ok(name_filter) = env::var("VALIDATE_RELOC") {
        use wasmparser::{BinaryReader, RelocSectionReader};

        let (_id, reloc_custom) = module
            .customs
            .iter()
            .find(|(_, c)| c.name() == "reloc.CODE")
            .expect("no reloc.CODE section - build with --emit-relocs");
        let data = reloc_custom.data(&Default::default());
        let reader = BinaryReader::new(&data, 0);
        let relocs: Vec<wasmparser::RelocationEntry> = RelocSectionReader::new(reader)
            .expect("parse reloc section header")
            .entries()
            .into_iter()
            .flatten()
            .collect();

        for func in module.funcs.iter() {
            let name = func.name.as_deref().unwrap_or("");
            if !name.contains(&name_filter) {
                continue;
            }
            let FunctionKind::Local(local) = &func.kind else {
                continue;
            };
            let Some(range) = &local.original_range else {
                continue;
            };
            println!("=== func {:?} name={} range={:?}", func.id(), name, range);

            struct Recorder {
                ordinal: usize,
                out: Vec<(usize, u32, String)>, // (ordinal, loc, debug string)
            }
            impl<'a> Visitor<'a> for Recorder {
                fn visit_instr(&mut self, instr: &ir::Instr, loc: &ir::InstrLocId) {
                    self.out
                        .push((self.ordinal, loc.data(), format!("{instr:?}")));
                    self.ordinal += 1;
                }
            }
            let mut rec = Recorder {
                ordinal: 0,
                out: Vec::new(),
            };
            dfs_in_order(&mut rec, local, local.entry_block());

            println!("  relocations in range:");
            for entry in &relocs {
                let rr = entry.relocation_range();
                if rr.start >= range.start && rr.end <= range.end {
                    println!(
                        "    ty={:?} offset={} index={} addend={} (offset-range.start={})",
                        entry.ty,
                        entry.offset,
                        entry.index,
                        entry.addend,
                        entry.offset as usize - range.start
                    );
                    // find nearest instruction by loc
                    if let Some((ord, loc, s)) = rec
                        .out
                        .iter()
                        .min_by_key(|(_, loc, _)| (*loc as i64 - entry.offset as i64).abs())
                    {
                        println!(
                            "      nearest instr: ordinal={ord} loc={loc} delta={} => {s}",
                            *loc as i64 - entry.offset as i64
                        );
                    }
                }
            }

            println!("  all instrs (ordinal, loc, instr):");
            for (ord, loc, s) in &rec.out {
                println!("    [{ord}] loc={loc} {s}");
            }
        }
        return;
    }

    if let Ok(id_str) = env::var("FUNC_ID") {
        let idx: usize = id_str.parse().unwrap();
        let needle = format!("idx: {idx} }}");
        let fid = module
            .funcs
            .iter()
            .find(|f| format!("{:?}", f.id()).ends_with(&needle))
            .map(|f| f.id());
        if let Some(fid) = fid {
            let func = module.funcs.get(fid);
            println!("=== func {:?} name={:?}", fid, func.name);
            println!("  ty={:?}", module.types.get(func.ty()));
            if let FunctionKind::Local(local) = &func.kind {
                struct Full {
                    out: Vec<String>,
                }
                impl<'a> Visitor<'a> for Full {
                    fn visit_instr(&mut self, instr: &ir::Instr, _loc: &ir::InstrLocId) {
                        self.out.push(format!("{instr:?}"));
                    }
                }
                let mut full = Full { out: Vec::new() };
                dfs_in_order(&mut full, local, local.entry_block());
                println!("  full dfs instrs: {}", full.out.len());
                for (i, s) in full.out.iter().enumerate() {
                    println!("    [{i}] {s}");
                }
            }
        }
        return;
    }

    if env::var("DATA_SEGMENTS").is_ok() {
        let mut active = 0;
        let mut passive = 0;
        for data in module.data.iter() {
            match &data.kind {
                walrus::DataKind::Active { offset, memory } => {
                    active += 1;
                    println!(
                        "{:?} ACTIVE memory={:?} offset={:?} len={}",
                        data.id(),
                        memory,
                        offset,
                        data.value.len()
                    );
                }
                walrus::DataKind::Passive => {
                    passive += 1;
                    println!("{:?} PASSIVE len={}", data.id(), data.value.len());
                }
            }
        }
        println!("total: {active} active, {passive} passive");
        // Also scan every function for memory.init / data.drop, which is
        // how passive segments actually get applied at runtime - a purely
        // static active-segment dump misses these entirely.
        for func in module.funcs.iter() {
            if let FunctionKind::Local(local) = &func.kind {
                struct BulkMemFinder {
                    hits: Vec<String>,
                }
                impl<'a> Visitor<'a> for BulkMemFinder {
                    fn visit_instr(&mut self, instr: &ir::Instr, _loc: &ir::InstrLocId) {
                        match instr {
                            ir::Instr::MemoryInit(_) | ir::Instr::DataDrop(_) => {
                                self.hits.push(format!("{instr:?}"));
                            }
                            _ => {}
                        }
                    }
                }
                let mut f = BulkMemFinder { hits: Vec::new() };
                dfs_in_order(&mut f, local, local.entry_block());
                if !f.hits.is_empty() {
                    println!(
                        "func {:?} name={:?} has bulk-mem ops: {:?}",
                        func.id(),
                        func.name,
                        f.hits
                    );
                }
            }
        }
        return;
    }

    if let Ok(addr_str) = env::var("DATA_AT") {
        let addr: u32 = addr_str.parse().unwrap();
        let len: usize = env::var("DATA_LEN")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(64);

        // Build one flat image across every active segment (segments can be
        // adjacent/contiguous - a pointer's pointee can straddle two of
        // them), matching how the runtime actually lays out linear memory.
        let mut max_end: u32 = 0;
        for data in module.data.iter() {
            if let walrus::DataKind::Active { offset, .. } = &data.kind {
                if let walrus::ConstExpr::Value(walrus::ir::Value::I32(off)) = offset {
                    max_end = max_end.max(*off as u32 + data.value.len() as u32);
                }
            }
        }
        let image_len = max_end.max(addr + len as u32) as usize;
        let mut image = vec![0u8; image_len];
        let mut covered = vec![false; image_len];
        for data in module.data.iter() {
            if let walrus::DataKind::Active { offset, .. } = &data.kind {
                if let walrus::ConstExpr::Value(walrus::ir::Value::I32(off)) = offset {
                    let off = *off as usize;
                    image[off..off + data.value.len()].copy_from_slice(&data.value);
                    for c in &mut covered[off..off + data.value.len()] {
                        *c = true;
                    }
                }
            }
        }

        let end = (addr as usize + len).min(image_len);
        let bytes = &image[addr as usize..end];
        let cov = &covered[addr as usize..end];
        println!("flat image len={image_len} - reading {addr}..{end}");
        println!("  bytes: {bytes:?}");
        println!("  covered (segment-backed, vs implicit zero): {cov:?}");
        println!("  as utf8 (lossy): {:?}", String::from_utf8_lossy(bytes));
        println!(
            "  nonzero count: {}/{}",
            bytes.iter().filter(|b| **b != 0).count(),
            bytes.len()
        );
        if bytes.len() >= 4 {
            println!(
                "  as u32 LE words: {:?}",
                bytes
                    .chunks(4)
                    .filter(|c| c.len() == 4)
                    .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect::<Vec<_>>()
            );
        }
        return;
    }

    if let Ok(name_filter) = env::var("RANGE_OF") {
        for func in module.funcs.iter() {
            let name = func.name.as_deref().unwrap_or("");
            if !name.contains(&name_filter) {
                continue;
            }
            if let FunctionKind::Local(local) = &func.kind {
                println!(
                    "{:?} name={} range={:?} len={:?}",
                    func.id(),
                    name,
                    local.original_range,
                    local.original_range.as_ref().map(|r| r.end - r.start)
                );
            }
        }
        return;
    }

    if let Ok(needle) = env::var("FIND_CONST") {
        let needle: i64 = needle.parse().unwrap();
        for func in module.funcs.iter() {
            if let FunctionKind::Local(local) = &func.kind {
                struct ConstFinder {
                    needle: i64,
                    hit: bool,
                    out: Vec<String>,
                }
                impl<'a> Visitor<'a> for ConstFinder {
                    fn visit_instr(&mut self, instr: &ir::Instr, _loc: &ir::InstrLocId) {
                        if let ir::Instr::Const(c) = instr {
                            let matched = match c.value {
                                ir::Value::I32(v) => v as i64 == self.needle,
                                ir::Value::I64(v) => v == self.needle,
                                _ => false,
                            };
                            if matched {
                                self.hit = true;
                            }
                        }
                        self.out.push(format!("{instr:?}"));
                    }
                }
                let mut finder = ConstFinder {
                    needle,
                    hit: false,
                    out: Vec::new(),
                };
                dfs_in_order(&mut finder, local, local.entry_block());
                if finder.hit {
                    println!(
                        "=== func {:?} name={:?} ({} total instrs)",
                        func.id(),
                        func.name,
                        finder.out.len()
                    );
                    for s in &finder.out {
                        println!("    {s}");
                    }
                }
            }
        }
        return;
    }

    if let Ok(slot_str) = env::var("WHAT_IS_AT_SLOT") {
        let slot: i32 = slot_str.parse().unwrap();
        for table in module.tables.iter() {
            for &elem_id in &table.elem_segments {
                let elem = module.elements.get(elem_id);
                let offset_val = match &elem.kind {
                    walrus::ElementKind::Active { offset, .. } => match offset {
                        walrus::ConstExpr::Value(walrus::ir::Value::I32(v)) => Some(*v),
                        _ => None,
                    },
                    _ => None,
                };
                if let (Some(off), walrus::ElementItems::Functions(fs)) = (offset_val, &elem.items)
                {
                    if slot >= off && (slot - off) < fs.len() as i32 {
                        let fid = fs[(slot - off) as usize];
                        println!(
                            "slot {slot} is in elem {:?} (base {off}) at position {}",
                            elem_id,
                            slot - off
                        );
                        let func = module.funcs.get(fid);
                        println!(
                            "  -> {:?} name={:?} kind={}",
                            fid,
                            func.name,
                            match &func.kind {
                                FunctionKind::Local(_) => "local",
                                FunctionKind::Import(_) => "import",
                                FunctionKind::Uninitialized(_) => "uninitialized",
                            }
                        );
                        println!("  ty={:?}", module.types.get(func.ty()));
                        if let FunctionKind::Local(local) = &func.kind {
                            struct Full {
                                out: Vec<String>,
                            }
                            impl<'a> Visitor<'a> for Full {
                                fn visit_instr(
                                    &mut self,
                                    instr: &ir::Instr,
                                    _loc: &ir::InstrLocId,
                                ) {
                                    self.out.push(format!("{instr:?}"));
                                }
                            }
                            let mut full = Full { out: Vec::new() };
                            dfs_in_order(&mut full, local, local.entry_block());
                            println!("  full dfs instrs: {}", full.out.len());
                            for s in &full.out {
                                println!("    {s}");
                            }
                        }
                    }
                }
            }
        }
        return;
    }

    if let Ok(name_filter) = env::var("FIND_TABLE_SLOT") {
        let matches: Vec<FunctionId> = module
            .funcs
            .iter()
            .filter(|f| f.name.as_deref().unwrap_or("").contains(&name_filter))
            .map(|f| f.id())
            .collect();
        println!("looking up table slot for funcs matching {name_filter:?} -> {matches:?}");
        let target_id = matches.first().copied();
        for table in module.tables.iter() {
            println!("table {:?} ty={:?}", table.id(), table.element_ty);
            for &elem_id in &table.elem_segments {
                let elem = module.elements.get(elem_id);
                let offset_val = match &elem.kind {
                    walrus::ElementKind::Active { offset, .. } => match offset {
                        walrus::ConstExpr::Value(walrus::ir::Value::I32(v)) => Some(*v),
                        _ => None,
                    },
                    _ => None,
                };
                if let walrus::ElementItems::Functions(fs) = &elem.items {
                    println!(
                        "  elem segment {:?} kind={:?} offset={:?} len={}",
                        elem_id,
                        elem.kind,
                        offset_val,
                        fs.len()
                    );
                    for (i, f) in fs.iter().enumerate() {
                        if Some(*f) == target_id {
                            let table_idx = offset_val.map(|o| o + i as i32);
                            println!(
                                "    ** FOUND at position {i} in this segment (table index = {table_idx:?}) **"
                            );
                        }
                    }
                }
            }
        }
        return;
    }

    let named = module.funcs.iter().filter(|f| f.name.is_some()).count();
    println!("named funcs: {named}");
    for func in module.funcs.iter().take(15) {
        println!("sample: {:?} name={:?}", func.id(), func.name);
    }

    for func in module.funcs.iter() {
        let name = func.name.as_deref().unwrap_or("");
        if !name.contains(&filter) {
            continue;
        }
        println!("=== func {:?} name={}", func.id(), name);
        println!("  ty={:?}", module.types.get(func.ty()));
        match &func.kind {
            FunctionKind::Local(local) => {
                let entry = local.block(local.entry_block());
                println!("  local, {} instrs in entry block", entry.instrs.len());
                if env::var("DUMP_INSTRS").is_ok() {
                    for (instr, _loc) in &entry.instrs {
                        println!("    {instr:?}");
                    }
                }

                struct Grapher {
                    calls: Vec<FunctionId>,
                    call_indirects: usize,
                    seen: HashSet<FunctionId>,
                }
                impl<'a> Visitor<'a> for Grapher {
                    fn visit_function_id(&mut self, function: &FunctionId) {
                        if self.seen.insert(*function) {
                            self.calls.push(*function);
                        }
                    }
                    fn visit_call_indirect(&mut self, _: &ir::CallIndirect) {
                        self.call_indirects += 1;
                    }
                }
                let mut g = Grapher {
                    calls: Vec::new(),
                    call_indirects: 0,
                    seen: HashSet::new(),
                };
                dfs_in_order(&mut g, local, local.entry_block());
                println!(
                    "  direct function refs: {}, call_indirect sites: {}",
                    g.calls.len(),
                    g.call_indirects
                );
                for callee in &g.calls {
                    let f = module.funcs.get(*callee);
                    let export = module
                        .exports
                        .iter()
                        .find(|e| matches!(e.item, walrus::ExportItem::Function(fid) if fid == *callee))
                        .map(|e| e.name.as_str());
                    println!(
                        "    -> {:?} name={:?} export={:?} kind={}",
                        callee,
                        f.name,
                        export,
                        match &f.kind {
                            FunctionKind::Local(_) => "local",
                            FunctionKind::Import(_) => "import",
                            FunctionKind::Uninitialized(_) => "uninitialized",
                        }
                    );
                }
            }
            FunctionKind::Import(_) => println!("  (import)"),
            FunctionKind::Uninitialized(_) => println!("  (uninitialized)"),
        }
    }
}
