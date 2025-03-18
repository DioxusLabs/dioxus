use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct JumpTable {
    /// The dylib containing the patch
    pub lib: PathBuf,

    /// old -> new
    /// does not take into account the base address of the patch when loaded into memory - need dlopen for that
    ///
    /// These are intended to be `*const ()` pointers but need to be `u64` for the hashmap. On 32-bit platforms
    /// you will need to cast to `usize` before using them.
    pub map: HashMap<u64, u64>,

    /// the address of the main function in the new original binary
    pub new_main_address: u64,

    /// the address of the main function in the old original binary
    pub old_main_address: u64,
}
