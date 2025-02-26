use anyhow::{Context, Result};
use itertools::Itertools;
use memmap::{Mmap, MmapOptions};
use object::{
    read::File, Architecture, BinaryFormat, Endianness, Object, ObjectSection, ObjectSymbol,
    Relocation, RelocationTarget, SectionIndex,
};
use std::{cmp::Ordering, ffi::OsStr, fs, ops::Deref, path::PathBuf};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
};
use tokio::process::Command;

use crate::Platform;

pub enum ReloadKind {
    /// An RSX-only patch
    Rsx,

    /// A patch that includes both RSX and binary assets
    Binary,

    /// A full rebuild
    Full,
}

#[derive(Debug, Clone)]
pub struct PatchData {
    pub direct_rustc: Vec<String>,
}

pub async fn attempt_partial_link(
    linker: PathBuf,
    work_dir: PathBuf,
    old_cache: PathBuf,
    new_cache: PathBuf,
    proc_main_addr: u64,
    patch_target: PathBuf,
    out_path: PathBuf,
) {
    let mut object = ObjectDiff::new(old_cache, new_cache).unwrap();
    object.load().unwrap();

    let all_exports = object
        .new
        .iter()
        .flat_map(|(_, f)| f.file.exports().unwrap())
        .map(|e| e.name())
        .collect::<HashSet<_>>();

    let mut adrp_imports = HashSet::new();

    let mut satisfied_exports = HashSet::new();

    let modified_symbols = object
        .modified_symbols
        .iter()
        .map(|f| f.as_str())
        .collect::<HashSet<_>>();

    if modified_symbols.is_empty() {
        println!("No modified symbols");
    }

    let mut modified_log = String::new();
    for m in modified_symbols.iter() {
        // if m.starts_with("l") {
        //     continue;
        // }

        let path = object.find_path_to_main(m);
        println!("m: {m}");
        println!("path: {path:#?}\n");
        modified_log.push_str(&format!("{m}\n"));
        modified_log.push_str(&format!("{path:#?}\n"));
    }

    let modified = object
        .modified_files
        .iter()
        .sorted_by(|a, b| a.0.cmp(&b.0))
        .collect::<Vec<_>>();

    // Figure out which symbols are required from *existing* code
    // We're going to create a stub `.o` file that satisfies these by jumping into the original code via a dynamic lookup / and or literally just manually doing it
    for fil in modified.iter() {
        let f = object
            .new
            .get(fil.0.file_name().unwrap().to_str().unwrap())
            .unwrap();

        for i in f.file.imports().unwrap() {
            if all_exports.contains(i.name()) {
                adrp_imports.insert(i.name());
            }
        }

        for e in f.file.exports().unwrap() {
            satisfied_exports.insert(e.name());
        }
    }

    // Remove any imports that are indeed satisifed
    for s in satisfied_exports.iter() {
        adrp_imports.remove(s);
    }

    // Assemble the stub
    let stub_data = make_stub_file(proc_main_addr, patch_target, adrp_imports);
    let stub_file = work_dir.join("stub.o");
    std::fs::write(&stub_file, stub_data).unwrap();

    let out = Command::new("cc")
        .args(modified.iter().map(|(f, _)| f))
        .arg(stub_file)
        .arg("-dylib")
        .arg("-Wl,-undefined,dynamic_lookup")
        .arg("-Wl,-unexported_symbol,_main")
        .arg("-arch")
        .arg("arm64")
        .arg("-dead_strip")
        .arg("-o")
        .arg(out_path)
        .output()
        .await
        .unwrap();

    let err = String::from_utf8_lossy(&out.stderr);
    println!("err: {err}");
    std::fs::write(work_dir.join("link_errs_partial.txt"), &*err).unwrap();

    // // -O0 ? supposedly faster
    // // -reproducible - even better?
    // // -exported_symbol and friends - could help with dead-code stripping
    // // -e symbol_name - for setting the entrypoint
    // // -keep_relocs ?

    // // run the linker, but unexport the `_main` symbol
    // let res = Command::new("cc")
    //     .args(object_files)
    //     .arg("-dylib")
    //     .arg("-undefined")
    //     .arg("dynamic_lookup")
    //     .arg("-Wl,-unexported_symbol,_main")
    //     .arg("-arch")
    //     .arg("arm64")
    //     .arg("-dead_strip") // maybe?
    //     .arg("-o")
    //     .arg(&out_file)
    //     .stdout(Stdio::piped())
    //     .stderr(Stdio::piped())
    //     .output()
    //     .await?;
}

/// todo: detect if the user specified a custom linker
fn system_linker(platform: Platform) -> &'static str {
    match platform {
        // mac + linux use just CC unless the user is trying to use something like mold / lld
        Platform::MacOS => "cc",
        Platform::Linux => "cc",
        Platform::Windows => "cc",
        Platform::Ios => "cc",
        Platform::Android => "cc",
        Platform::Server => "cc",
        Platform::Liveview => "cc",
        Platform::Web => "wasm-ld",
    }
}

struct ObjectDiff {
    old: BTreeMap<String, LoadedFile>,
    new: BTreeMap<String, LoadedFile>,
    modified_files: HashMap<PathBuf, HashSet<String>>,
    modified_symbols: HashSet<String>,
    parents: HashMap<String, HashSet<String>>,
}

impl ObjectDiff {
    fn new(old_cache: PathBuf, new_cache: PathBuf) -> Result<Self> {
        Ok(Self {
            old: LoadedFile::from_dir(&old_cache)?,
            new: LoadedFile::from_dir(&new_cache)?,
            modified_files: Default::default(),
            modified_symbols: Default::default(),
            parents: Default::default(),
        })
    }

    fn load(&mut self) -> Result<()> {
        let num_right = self.new.len();

        let keys = self.new.keys().cloned().collect::<Vec<_>>();
        for (idx, f) in keys.iter().enumerate() {
            println!("----- {:?} {}/{} -----", f, idx, num_right);

            let changed_before = self.modified_symbols.len();
            self.load_file(f)?;
            let changed_after = self.modified_symbols.len();

            if changed_after > changed_before {
                println!("❌ -> {}", changed_after - changed_before);
            }
        }

        Ok(())
    }

    /// Walk the call  to find the path to the main function
    fn find_path_to_main(&self, name: &str) -> Vec<String> {
        let mut path = Vec::new();
        let mut visited = std::collections::HashSet::new();

        // Helper function for DFS with backtracking
        fn dfs(
            current: &str,
            path: &mut Vec<String>,
            visited: &mut std::collections::HashSet<String>,
            parents: &std::collections::HashMap<String, HashSet<String>>,
        ) -> bool {
            // If we've found main, we're done
            if current.ends_with("_main") {
                path.push(current.to_string());
                return true;
            }

            // Mark current node as visited
            visited.insert(current.to_string());
            path.push(current.to_string());

            // Check all parents of the current node
            if let Some(parent_nodes) = parents.get(current) {
                for parent in parent_nodes {
                    if !visited.contains(parent) {
                        if dfs(parent, path, visited, parents) {
                            return true;
                        }
                    }
                }
            }

            // If no path is found through this node, backtrack
            path.pop();

            false
        }

        // Start DFS from the given name
        dfs(name, &mut path, &mut visited, &self.parents);

        path
    }

    fn load_file(&mut self, name: &str) -> Result<()> {
        let new = &self.new[name];
        let Some(old) = self.old.get(name) else {
            self.modified_files.entry(new.path.clone()).or_default();
            return Ok(());
        };

        let mut changed_list = HashSet::new();
        for section in new.file.sections() {
            let n = section.name().unwrap();
            if n == "__text"
                || n == "__const"
                || n.starts_with("__literal")
                || n == "__eh_frame"
                || n == "__compact_unwind"
                || n == "__gcc_except_tab"
                || n == "__common"
                || n == "__bss"
            {
                changed_list.extend(self.accumulate_changed(&old, &new, section.index()));
            } else {
                println!("Skipping section: {n}");
            }
        }

        for c in changed_list.iter() {
            if !c.starts_with("l") && !c.starts_with("ltmp") {
                self.modified_symbols.insert(c.to_string());
            } else {
                let mod_name = format!("{c}_{name}");
                self.modified_symbols.insert(mod_name);
            }
        }

        for (child, parents) in new.parents.iter() {
            let child_name = match child.starts_with("l") {
                true => format!("{child}_{name}"),
                false => child.to_string(),
            };

            for parent in parents {
                let p_name = match parent.starts_with("l") {
                    true => format!("{parent}_{name}"),
                    false => parent.to_string(),
                };

                self.parents
                    .entry(child_name.clone())
                    .or_default()
                    .insert(p_name);
            }
        }

        Ok(())
    }

    fn accumulate_changed(
        &self,
        old: &LoadedFile,
        new: &LoadedFile,
        section_idx: SectionIndex,
    ) -> HashSet<&'static str> {
        let mut local_modified = HashSet::new();

        // Accumulate modified symbols using masking in functions
        let relocated_new = acc_symbols(&new.file, section_idx);
        let mut relocated_old = acc_symbols(&old.file, section_idx)
            .into_iter()
            .map(|f| (f.name, f))
            .collect::<HashMap<_, _>>();

        for right in relocated_new {
            let Some(left) = relocated_old.remove(right.name) else {
                local_modified.insert(right.name);
                continue;
            };

            // If the contents of the assembly changed, track it
            if !compare_masked(old.file, new.file, &left, &right) {
                local_modified.insert(left.name);
                local_modified.insert(right.name);
            }
        }

        local_modified
    }
}

/// A file loaded into memory with its analysis
///
/// We leak the module to make it easier to deal with its contents
struct LoadedFile {
    path: PathBuf,
    open_file: std::fs::File,
    mmap: &'static Mmap,

    file: &'static File<'static>,

    // symbol -> symbols
    parents: HashMap<&'static str, HashSet<&'static str>>,
}

impl LoadedFile {
    fn from_dir(dir: &Path) -> anyhow::Result<BTreeMap<String, Self>> {
        std::fs::read_dir(dir)?
            .into_iter()
            .flatten()
            .filter(|e| e.path().extension() == Some(OsStr::new("o")))
            .map(|e| {
                Ok((
                    e.path().file_name().unwrap().to_string_lossy().to_string(),
                    Self::new(e.path())?,
                ))
            })
            .collect()
    }

    fn new(path: PathBuf) -> anyhow::Result<Self> {
        let open_file = std::fs::File::open(&path)?;
        let mmap = unsafe { MmapOptions::new().map(&open_file).unwrap() };
        let mmap: &'static Mmap = Box::leak(Box::new(mmap));
        let f = File::parse(mmap.deref() as &[u8])?;
        let file: &'static File<'static> = Box::leak(Box::new(f));

        // Set up the data structures
        let mut sym_tab = HashMap::<&'static str, RelocatedSymbol<'static>>::new();
        let mut parents = HashMap::<&'static str, HashSet<&'static str>>::new();

        // Build the symbol table
        for sect in file.sections() {
            for r in acc_symbols(&file, sect.index()) {
                sym_tab.insert(r.name, r);
            }
        }

        // Create a map of address -> symbol so we can resolve the section of a symbol
        let local_defs = file
            .symbols()
            .filter(|s| s.is_definition())
            .map(|s| (s.address(), s.name().unwrap()))
            .collect::<BTreeMap<_, _>>();

        // Build the call graph by walking the relocations
        // We keep track of what calls whata
        for (sym_name, sym) in sym_tab.iter() {
            let sym_section = file.section_by_index(sym.section).unwrap();
            let sym_data = sym_section.data().unwrap();

            for (addr, reloc) in sym.relocations.iter() {
                let target = match symbol_name_of_relo(file, reloc.target()) {
                    Some(name) => name,
                    None => {
                        let addend = u64::from_le_bytes(
                            sym_data[*addr as usize..(*addr + 8) as usize]
                                .try_into()
                                .unwrap(),
                        );
                        local_defs.get(&addend).unwrap()
                    }
                };

                parents.entry(target).or_default().insert(sym_name);
            }
        }

        Ok(Self {
            path,
            open_file,
            mmap,
            file,
            parents,
        })
    }
}

/// A function with its relevant relocations to be used for masked comparisons
struct RelocatedSymbol<'a> {
    name: &'a str,
    /// offset within the section
    offset: usize,
    data: &'a [u8],
    relocations: &'a [(u64, Relocation)],
    sym: object::Symbol<'a, 'a>,
    section: SectionIndex,
}

fn acc_symbols<'a>(new: &'a File<'a>, section_idx: SectionIndex) -> Vec<RelocatedSymbol<'a>> {
    let mut syms = vec![];

    let section = new.section_by_index(section_idx).unwrap();

    let sorted = new
        .symbols()
        .filter(|s| s.section_index() == Some(section_idx))
        .sorted_by(|a, b| {
            let addr = a.address().cmp(&b.address());
            if addr == Ordering::Equal {
                a.index().0.cmp(&b.index().0)
            } else {
                addr
            }
        })
        .collect::<Vec<_>>();

    // todo!!!!!! jon: don't leak this lol
    let relocations = section
        .relocations()
        .sorted_by(|a, b| a.0.cmp(&b.0).reverse())
        .collect::<Vec<_>>()
        .leak();

    let data = section.data().unwrap();

    // No symbols, no symbols,
    if sorted.is_empty() {
        return vec![];
    }

    // The end of the currently analyzed function
    let mut func_end = section.size() as usize;

    // The idx into the relocation list that applies to this function. We'll march these
    let mut reloc_idx = 0;

    // Walk in reverse so we can use the text_length as the initial backstop and to match relocation order
    for sym in sorted.into_iter().rev() {
        let sym_offset = sym.address() - section.address();

        // Move the head/tail to include the sub-slice of the relocations that apply to this symbol
        let mut reloc_start = None;
        loop {
            // If we've reached the end of the relocations then we're done
            if reloc_idx == relocations.len() {
                break;
            }

            // relocations behind the symbol start don't apply
            if relocations[reloc_idx].0 < sym_offset {
                break;
            }

            // Set the head to the first relocation that applies
            if reloc_start.is_none() {
                reloc_start = Some(reloc_idx);
            }

            reloc_idx += 1;
        }

        // Identify the instructions that apply to this symbol
        let data = match reloc_start {
            Some(_start) => &data[sym_offset as usize..func_end],
            _ => &[],
        };

        // Identify the relocations that apply to this symbol
        let relocations = match reloc_start {
            Some(start) => &relocations[start..reloc_idx],
            None => &[],
        };

        syms.push(RelocatedSymbol {
            name: sym.name().unwrap(),
            sym,
            offset: sym_offset as usize,
            data,
            relocations,
            section: section_idx,
        });

        func_end = sym_offset as usize;
    }

    assert_eq!(reloc_idx, relocations.len());

    syms
}

/// Compare two sets of bytes, masking out the bytes that are not part of the symbol
/// This is so we can compare functions with different relocations
fn compare_masked<'a>(
    old: &impl Object<'a>,
    new: &impl Object<'a>,
    left: &RelocatedSymbol,
    right: &RelocatedSymbol,
) -> bool {
    // Make sure the relocations are the same length
    if left.relocations.len() != right.relocations.len() {
        return false;
    }

    // Make sure the data is the same length
    // If the size changed then the instructions are different (well, not necessarily, but enough)
    if left.data.len() != right.data.len() {
        return false;
    }

    // Make sure the names match
    if left.name != right.name {
        return false;
    }

    // We're going to walk from relocation target to target, but since there's no implicit target
    // to start with, we simply use the end of the data
    let mut last = left.data.len();

    // Ensure the relocations point to the same symbol
    // Data symbols are special ... todo
    //
    // relocations are in reverse order, so we can also compare the data as we go
    for x in 0..left.relocations.len() {
        // Grab the reloc
        let (l_addr, left_reloc): &(u64, Relocation) = &left.relocations[x];
        let (_r_addr, right_reloc): &(u64, Relocation) = &right.relocations[x];

        // The targets might not be same by index but should resolve to the same *name*
        let left_target: RelocationTarget = left_reloc.target();
        let right_target: RelocationTarget = right_reloc.target();

        // Use the name of the symbol to compare
        // todo: decide if it's internal vs external
        let left_name = symbol_name_of_relo(old, left_target);
        let right_name = symbol_name_of_relo(new, right_target);
        let (Some(left_name), Some(right_name)) = (left_name, right_name) else {
            continue;
        };

        // Make sure the names match
        // if the target is a locally defined symbol, then it might be the same
        // todo(jon): hash the masked contents
        if left_name != right_name {
            return false;
        }

        // Check the data
        // the slice is the end of the relocation to the start of the previous relocation
        let reloc_byte_size = (left_reloc.size() as usize) / 8;
        let start = *l_addr as usize - left.offset as usize + reloc_byte_size;

        // Some relocations target the same location
        // In these cases, we just continue since we just masked and checked them already
        if (*l_addr as usize - left.offset as usize) == last {
            continue;
        }

        debug_assert!(start <= last);
        debug_assert!(start <= left.data.len());

        if &left.data[start..last] != &right.data[start..last] {
            return false;
        }

        if left_reloc.flags() != right_reloc.flags() {
            return false;
        }

        // todo: more checking... the symbols might be local
        last = start - reloc_byte_size;
    }

    // And a final check to make sure the data is the same
    if left.data[..last] != right.data[..last] {
        return false;
    }

    true
}

fn symbol_name_of_relo<'a>(obj: &impl Object<'a>, target: RelocationTarget) -> Option<&'a str> {
    match target {
        RelocationTarget::Symbol(symbol_index) => obj
            .symbol_by_index(symbol_index)
            .unwrap()
            .name_bytes()
            .ok()
            .and_then(|s| std::str::from_utf8(s).ok()),
        RelocationTarget::Section(_) => None,
        RelocationTarget::Absolute => None,
        _ => None,
    }
}

fn make_stub_file(
    proc_main_addr: u64,
    patch_target: PathBuf,
    adrp_imports: HashSet<&[u8]>,
) -> Vec<u8> {
    let data = fs::read(&patch_target).unwrap();
    let old = File::parse(&data as &[u8]).unwrap();
    let main_sym = old.symbol_by_name_bytes(b"_main").unwrap();
    let aslr_offset = proc_main_addr - main_sym.address();
    let addressed = old
        .symbols()
        .filter_map(|sym| {
            adrp_imports
                .get(sym.name_bytes().ok()?)
                .copied()
                .map(|o| (o, sym.address() + aslr_offset))
        })
        .collect::<HashMap<_, _>>();

    build_stub(
        old.format(),
        old.architecture(),
        old.endianness(),
        addressed,
    )
    .unwrap()
}

/// Builds an object file that satisfies the imports
///
/// Creates stub functions that jump to known addresses in a target process.
///
/// .section __TEXT,__text
/// .globl __ZN4core3fmt3num52_$LT$impl$u20$core..fmt..Debug$u20$for$u20$usize$GT$3fmt17h4e710f94be547818E
/// .p2align 2
/// __ZN4core3fmt3num52_$LT$impl$u20$core..fmt..Debug$u20$for$u20$usize$GT$3fmt17h4e710f94be547818E:
///     // Load 64-bit address using immediate values
///     movz x9, #0xCDEF          // Bottom 16 bits
///     movk x9, #0x89AB, lsl #16 // Next 16 bits
///     movk x9, #0x4567, lsl #32 // Next 16 bits
///     movk x9, #0x0123, lsl #48 // Top 16 bits
///
///     // Branch to the loaded address
///     br x9
fn build_stub(
    format: BinaryFormat,
    architecture: Architecture,
    endian: Endianness,
    adrp_imports: HashMap<&[u8], u64>,
) -> Result<Vec<u8>> {
    use object::{
        write::{Object, Symbol, SymbolSection},
        SectionKind, SymbolFlags, SymbolKind, SymbolScope,
    };

    // Create a new ARM64 object file
    let mut obj = Object::new(format, architecture, endian);

    // Add a text section for our trampolines
    let text_section = obj.add_section(Vec::new(), ".text".into(), SectionKind::Text);

    // For each symbol, create a trampoline that loads the immediate address and jumps to it
    for (name, addr) in adrp_imports {
        let mut trampoline = Vec::new();

        // todo: writing these bytes are only good for arm64
        //
        //
        // Break down the 64-bit address into 16-bit chunks
        let addr0 = (addr & 0xFFFF) as u16; // Bits 0-15
        let addr1 = ((addr >> 16) & 0xFFFF) as u16; // Bits 16-31
        let addr2 = ((addr >> 32) & 0xFFFF) as u16; // Bits 32-47
        let addr3 = ((addr >> 48) & 0xFFFF) as u16; // Bits 48-63

        // MOVZ x9, #addr0
        let movz = 0xD2800009 | ((addr0 as u32) << 5);
        trampoline.extend_from_slice(&movz.to_le_bytes());

        // MOVK x9, #addr1, LSL #16
        let movk1 = 0xF2A00009 | ((addr1 as u32) << 5);
        trampoline.extend_from_slice(&movk1.to_le_bytes());

        // MOVK x9, #addr2, LSL #32
        let movk2 = 0xF2C00009 | ((addr2 as u32) << 5);
        trampoline.extend_from_slice(&movk2.to_le_bytes());

        // MOVK x9, #addr3, LSL #48
        let movk3 = 0xF2E00009 | ((addr3 as u32) << 5);
        trampoline.extend_from_slice(&movk3.to_le_bytes());

        // BR x9 - Branch to the address in x9
        let br: u32 = 0xD61F0120;
        trampoline.extend_from_slice(&br.to_le_bytes());

        // Add the trampoline to the text section
        let symbol_offset = obj.append_section_data(text_section, &trampoline, 4);

        // we are writing this:
        // __ZN93_$LT$generational_box..references..GenerationalRef$LT$R$GT$$u20$as$u20$core..fmt..Display$GT$3fmt17h455abb35572b9c11E
        //
        // but we should be writing this:
        //       _$LT$generational_box..references..GenerationalRef$LT$R$GT$$u20$as$u20$core..fmt..Display$GT$::fmt::h455abb35572b9c11
        // let name = strip_mangled(name);

        let name = if name.starts_with(b"_") {
            &name[1..]
        } else {
            name
        };

        // Add the symbol
        obj.add_symbol(Symbol {
            name: name.into(),
            value: symbol_offset,
            size: trampoline.len() as u64,
            kind: SymbolKind::Text,
            scope: SymbolScope::Dynamic,
            weak: false,
            section: SymbolSection::Section(text_section),
            flags: SymbolFlags::None,
        });
    }

    obj.write().context("Failed to write object file")
}
