//! Lift hidden symbols to the dlsym table so they can be dynamically linked against
//!
//! This enables rapid builds and hotpatching
//!
//! We link together the rlibs with a file that creates a dynamic library out of them.

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
pub use subsecond_types::*;
use target_lexicon::{OperatingSystem, Triple};
use tokio::process::Command;

#[test]
fn load_object_file() {
    // let data = include_bytes!("../../data/subsecond-harness-fat");
    // let object = File::parse(data.deref() as &[u8]).unwrap();

    // // We're going to cherrypick our symbols from the object file into the dynamic symbol table
    // let mut out =
    //     object::write::Object::new(object.format(), object.architecture(), object.endianness());

    // for s in object.symbols() {
    //     let import = out.add_symbol(object::write::Symbol {
    //         name: todo!(),
    //         value: todo!(),
    //         size: todo!(),
    //         kind: todo!(),
    //         scope: todo!(),
    //         weak: todo!(),
    //         section: todo!(),
    //         flags: todo!(),
    //     });
    //     out.add_symbol(object::write::Symbol {
    //         name: todo!(),
    //         value: todo!(),
    //         size: todo!(),
    //         kind: todo!(),
    //         scope: todo!(),
    //         weak: todo!(),
    //         section: todo!(),
    //         flags: todo!(),
    //     });
    // }

    // let writer = object::write
    // let dyn_syms = object.dynamic_symbols();

    // object.dynamic_symbol_table().unwrap()
    // // println!("dun asd");
    // for s in dyn_syms {
    //     println!("sym: {:?}", s)
    // }

    // let mut symbols = object
    //     .symbols()
    //     // .filter(|s| s.is_definition() && s.is_global())
    //     .collect::<Vec<_>>();

    // println!("There are {:?} symbols", symbols.len());

    // // convert the symbols to protected visibility if they come from rust land, and then put them in the dynamic symbol table
    // for sym in symbols.iter_mut().take(100) {
    //     object.dynamic_symbol_table()
    // }
}
