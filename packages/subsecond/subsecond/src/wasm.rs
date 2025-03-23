use super::*;
use js_sys::Uint32Array;
use subsecond_types::AddressMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn __patch_wasm(pointers: Uint32Array) {
    let mut table = JumpTable {
        aslr_reference: 0,
        lib: PathBuf::from("patch.wasm"),
        map: AddressMap::default(),
        new_base_address: 0,
        old_base_address: 0,
    };

    for x in 0..(pointers.length() / 2) {
        let left = pointers.get_index(x);
        let right = pointers.get_index(x + 1);
        table.map.insert(left as u64, right as u64);
    }
}
