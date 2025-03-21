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

pub mod partial;
pub mod resolve;

pub fn create_jump_table(
    original: &Path,
    patch: &Path,
    triple: &Triple,
) -> anyhow::Result<JumpTable> {
    let obj1_bytes = std::fs::read(original).context("Could not read original file")?;
    let obj2_bytes = std::fs::read(patch).context("Could not read patch file")?;
    let obj1 = File::parse(&obj1_bytes as &[u8]).unwrap();
    let obj2 = File::parse(&obj2_bytes as &[u8]).unwrap();

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

    tracing::debug!("old_name_to_addr: {:#?}", old_name_to_addr);
    tracing::debug!("new_name_to_addr: {:#?}", new_name_to_addr);

    // on windows there is no symbol so we leave the old address as 0
    // on wasm there is no ASLR so we leave the old address as 0
    let mut old_base_address = 0;
    let mut new_base_address = 0;
    match triple.operating_system {
        OperatingSystem::Darwin(_)
        | OperatingSystem::Linux
        | OperatingSystem::MacOSX(_)
        | OperatingSystem::IOS(_)
        | OperatingSystem::Windows => {
            let options = ["___rust_alloc", "__rust_alloc"];
            for option in options {
                if old_name_to_addr.contains_key(option) {
                    old_base_address = old_name_to_addr.get(option).unwrap().clone();
                    new_base_address = new_name_to_addr.get(option).unwrap().clone();
                    break;
                }
            }
        }
        _ => {}
    }

    for (new_name, new_addr) in new_name_to_addr {
        if let Some(old_addr) = old_name_to_addr.get(new_name) {
            map.insert(*old_addr, new_addr);
        }
    }

    let aslr_reference = old_name_to_addr
        .get("aslr_reference")
        .unwrap_or_else(|| {
            old_name_to_addr
                .get("_aslr_reference")
                .expect("failed to find aslr_reference")
        })
        .clone();

    if new_base_address == 0 {
        panic!("new_base_address is 0");
    }

    Ok(JumpTable {
        lib: patch.to_path_buf(),
        map,
        old_base_address,
        new_base_address,
        aslr_reference,
    })
}
