use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    hash::{BuildHasherDefault, Hasher},
    path::PathBuf,
};

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

    /// the address of the base address of the old original binary
    ///
    /// machos: this is the address of the `_mh_execute_header` symbol usually at 0x100000000 and loaded near 0x100000000
    /// linux: this is the address of the `__executable_start` symbol usually at 0x0 but loaded around 0x555555550000
    /// windows: this is the address of the `ImageBase` field of the PE header
    /// wasm: not useful since there's no ASLR
    ///
    /// While we can generally guess that these values are, it's possible they are different and thus reading
    /// them dynamically is worthwhile.
    pub aslr_reference: u64,

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

    /// The amount of ifuncs this will register. This is used by WASM to know how much space to allocate
    /// for the ifuncs in the ifunc table
    pub ifunc_count: u64,
}

/// An address to address hashmap that does not hash addresses since addresses are by definition unique.
pub type AddressMap = HashMap<u64, u64, BuildAddressHasher>;
pub type BuildAddressHasher = BuildHasherDefault<AddressHasher>;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AddressHasher(u64);
impl Hasher for AddressHasher {
    fn write(&mut self, _: &[u8]) {
        panic!("Invalid use of NoHashHasher")
    }
    fn write_u8(&mut self, n: u8) {
        self.0 = u64::from(n)
    }
    fn write_u16(&mut self, n: u16) {
        self.0 = u64::from(n)
    }
    fn write_u32(&mut self, n: u32) {
        self.0 = u64::from(n)
    }
    fn write_u64(&mut self, n: u64) {
        self.0 = n
    }
    fn write_usize(&mut self, n: usize) {
        self.0 = n as u64
    }
    fn write_i8(&mut self, n: i8) {
        self.0 = n as u64
    }
    fn write_i16(&mut self, n: i16) {
        self.0 = n as u64
    }
    fn write_i32(&mut self, n: i32) {
        self.0 = n as u64
    }
    fn write_i64(&mut self, n: i64) {
        self.0 = n as u64
    }
    fn write_isize(&mut self, n: isize) {
        self.0 = n as u64
    }
    fn finish(&self) -> u64 {
        self.0
    }
}
