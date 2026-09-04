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

    #[serde(default)]
    pub wasm_layout: Option<WasmPatchLayout>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WasmPatchLayout {
    pub memory_size: u32,
    pub memory_alignment: u32,
    pub table_size: u32,
    pub table_alignment: u32,
}

impl WasmPatchLayout {
    pub const PAGE_SIZE: u64 = 65536;

    pub fn reservation(&self) -> Option<(u32, u32)> {
        let memory_alignment = 1u64.checked_shl(self.memory_alignment)?;
        let table_alignment = 1u32.checked_shl(self.table_alignment)?;
        if self.memory_alignment > 31 {
            return None;
        }
        let pages = 1
            + u64::from(self.memory_size).div_ceil(Self::PAGE_SIZE)
            + memory_alignment.saturating_sub(Self::PAGE_SIZE) / Self::PAGE_SIZE;
        if pages > 65536 {
            return None;
        }
        Some((
            pages as u32,
            self.table_size.checked_add(table_alignment - 1)?,
        ))
    }

    pub fn memory_base(&self, previous_pages: u32) -> Option<u32> {
        let address = (u64::from(previous_pages) + 1).checked_mul(Self::PAGE_SIZE)?;
        let alignment = 1u64.checked_shl(self.memory_alignment)?;
        u32::try_from(address.checked_add(alignment - 1)? & !(alignment - 1)).ok()
    }

    pub fn table_base(&self, previous_slots: u32) -> Option<u32> {
        let alignment = 1u32.checked_shl(self.table_alignment)?;
        Some(previous_slots.checked_add(alignment - 1)? & !(alignment - 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_reservations_cover_bss_and_alignment() {
        for memory_alignment in [0, 4, 16, 20] {
            for table_alignment in [0, 2, 5] {
                let layout = WasmPatchLayout {
                    memory_size: 1048580,
                    memory_alignment,
                    table_size: 7,
                    table_alignment,
                };
                let (pages, slots) = layout.reservation().unwrap();
                for previous in [0, 1, 3, 16, 123] {
                    let base = u64::from(layout.memory_base(previous).unwrap());
                    assert_eq!(base % (1u64 << memory_alignment), 0);
                    assert!(
                        base + u64::from(layout.memory_size)
                            <= u64::from(previous + pages) * WasmPatchLayout::PAGE_SIZE
                    );
                    let base = layout.table_base(previous).unwrap();
                    assert_eq!(base % (1u32 << table_alignment), 0);
                    assert!(base + layout.table_size <= previous + slots);
                }
            }
        }
    }

    #[test]
    fn wasm_reservations_reject_overflow() {
        assert!(
            WasmPatchLayout {
                memory_alignment: 32,
                ..Default::default()
            }
            .reservation()
            .is_none()
        );
        assert!(
            WasmPatchLayout {
                table_alignment: 32,
                ..Default::default()
            }
            .reservation()
            .is_none()
        );
        assert!(
            WasmPatchLayout {
                memory_size: u32::MAX,
                ..Default::default()
            }
            .reservation()
            .is_none()
        );
        assert!(WasmPatchLayout::default().memory_base(65535).is_none());
    }
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
