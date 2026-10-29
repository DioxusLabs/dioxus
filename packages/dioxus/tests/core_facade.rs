use dioxus::prelude::*;

#[test]
fn vdom_paths_are_preserved() {
    assert_eq!(
        std::any::TypeId::of::<dioxus::core::VirtualDom>(),
        std::any::TypeId::of::<dioxus::dioxus_vdom::VirtualDom>()
    );
    let _: Option<Element> = None;
    #[cfg(feature = "html")]
    let _: Option<dioxus::events::MouseEvent> = None;
    let selected = dioxus::config_macros::maybe_wasm_split! {
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
    #[cfg(feature = "wasm-split")]
    assert_eq!(
        std::any::TypeId::of::<dioxus::wasm_split::LazyLoader<(), ()>>(),
        std::any::TypeId::of::<wasm_split::LazyLoader<(), ()>>()
    );
    #[cfg(all(
        feature = "macro",
        feature = "signals",
        feature = "hooks",
        feature = "html"
    ))]
    let _ = App;
}

#[cfg(feature = "document")]
use dioxus::document as _;
#[cfg(feature = "hooks")]
use dioxus::hooks as _;
#[cfg(feature = "html")]
use dioxus::html as _;
#[cfg(feature = "ssr")]
use dioxus::ssr as _;
#[cfg(feature = "signals")]
use dioxus::{signals as _, stores as _};

#[cfg(all(
    feature = "macro",
    feature = "signals",
    feature = "hooks",
    feature = "html"
))]
#[dioxus::core_macro::component]
fn App() -> Element {
    let count = use_signal(|| 0);
    rsx! { div { "{count}" } }
}
