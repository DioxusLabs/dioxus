#[test]
fn config_macro_is_always_available() {
    let selected = dioxus_core::config_macros::maybe_wasm_split! {
        if wasm_split { 1 } else { 2 }
    };
    assert_eq!(
        selected,
        if cfg!(all(feature = "wasm-split", target_arch = "wasm32")) {
            1
        } else {
            2
        }
    );
}

#[cfg(feature = "wasm-split")]
#[test]
fn splitter_is_available_at_root_and_in_prelude() {
    use dioxus_core::prelude::wasm_split;
    assert_eq!(
        std::any::TypeId::of::<dioxus_core::wasm_split::LazyLoader<(), ()>>(),
        std::any::TypeId::of::<wasm_split::LazyLoader<(), ()>>()
    );
}
