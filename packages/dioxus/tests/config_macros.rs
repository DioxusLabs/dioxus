#[cfg(feature = "config-macros")]
#[test]
fn config_macro_selects_the_target_branch() {
    let split = dioxus::config_macros::maybe_wasm_split! {
        if wasm_split { { true } } else { { false } }
    };
    assert_eq!(
        split,
        cfg!(all(feature = "wasm-split", target_arch = "wasm32"))
    );
}

#[cfg(all(feature = "macro", not(target_arch = "wasm32")))]
#[test]
fn lazy_component_works_without_wasm_splitting() {
    use dioxus::prelude::*;

    #[component(lazy)]
    fn LazyComponent() -> Element {
        VNode::empty()
    }

    assert!(LazyComponent(()).is_ok());
}
