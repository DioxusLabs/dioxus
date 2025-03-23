use super::*;
use js_sys::Uint32Array;
use subsecond_types::AddressMap;
use wasm_bindgen::prelude::*;

use super::*;

#[wasm_bindgen]
pub fn __patch_wasm(pointers: Uint32Array) {
    let mut table = JumpTable {
        aslr_reference: 0,
        lib: PathBuf::from("patch.wasm"),
        map: AddressMap::default(),
        new_base_address: 0,
        old_base_address: 0,
    };

    tracing::info!("Patching wasm with {:?}", pointers);

    let mut idx = 0;
    for _ in 0..pointers.length() {
        let left = pointers.get_index(idx);
        let right = pointers.get_index(idx + 1);
        tracing::info!("Adding pointer {:?} -> {:?}", left, right);
        table.map.insert(left as u64, right as u64);
        idx += 2
    }

    unsafe {
        APP_JUMP_TABLE = Some(table);
        CHANGED = true;
        HOTRELOAD_HANDLERS.clone().iter().for_each(|handler| {
            handler();
        });
    }
}
