use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod nohasher;
pub use nohasher::AddressMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct JumpTable {
    /// The dylib containing the patch. This should be a valid path so you can just pass it to LibLoading
    ///
    /// On wasm you will need to fetch() this file and then pass it to the WebAssembly.instantiate() function
    pub lib: PathBuf,

    /// old -> new
    /// does not take into account the base address of the patch when loaded into memory - need dlopen for that
    ///
    /// These are intended to be `*const ()` pointers but need to be `u64` for the hashmap. On 32-bit platforms
    /// you will need to cast to `usize` before using them.
    pub map: AddressMap,

    /// Global offset table required to relocate the data segment
    ///
    /// This is used mainly for WASM to relocate the data segment to the correct address
    pub got: Vec<(String, u64)>,

    /// The location of the aslr reference in the binary, used to calcualte offsets for the jump table
    pub aslr_reference: u64,

    /// the address of the base address of the old original binary
    ///
    /// machos: this is the address of the `_mh_execute_header` symbol usually at 0x100000000 and loaded near 0x100000000
    /// linux: this is the address of the `__executable_start` symbol usually at 0x0 but loaded around 0x555555550000
    /// windows: this is the address of the `ImageBase` field of the PE header
    /// wasm: not useful since there's no ASLR
    ///
    /// While we can generally guess that these values are, it's possible they are different and thus reading
    /// them dynamically is worthwhile.
    pub old_base_address: u64,

    /// the address of the base address of the new binary
    ///
    /// machos: this is the address of the `_mh_execute_header` symbol usually at 0x100000000 and loaded near 0x100000000
    /// linux: this is the address of the `__executable_start` symbol usually at 0x0 but loaded around 0x555555550000
    /// windows: this is the address of the `ImageBase` field of the PE header
    /// wasm: not useful since there's no ASLR
    ///
    /// While we can generally guess that these values are, it's possible they are different and thus reading
    /// them dynamically is worthwhile.
    pub new_base_address: u64,
}
