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
