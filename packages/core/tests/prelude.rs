#![cfg(all(
    feature = "macro",
    feature = "signals",
    feature = "hooks",
    feature = "stores",
    feature = "html"
))]

use dioxus_core::prelude::*;

dioxus_core::html::define_elements! {
    #[element(name = "custom-panel")]
    customPanel { metric }
}

#[derive(Clone, Store)]
struct Counter {
    value: i32,
}

#[component]
fn App(name: String) -> Element {
    let count = use_signal(|| 0);
    rsx! { customPanel { metric: "value", div { "{name}: {count}" } } }
}

#[test]
fn prelude_supports_components_and_store_derives() {
    assert_eq!(Counter { value: 1 }.value, 1);
    let _ = App;
}
