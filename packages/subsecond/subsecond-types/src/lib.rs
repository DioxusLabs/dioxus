use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct JumpTable {
    // old -> new
    // does not take into account the base address of the patch when loaded into memory - need dlopen for that
    pub map: HashMap<u64, u64>,

    /// the address of the main function in the new original binary
    pub new_main_address: u64,

    /// the address of the main function in the old original binary
    pub old_main_address: u64,
}
