#![cfg(all(
    feature = "macro",
    feature = "signals",
    feature = "hooks",
    feature = "stores",
    feature = "html"
))]

use dioxus_core::prelude::*;

#[derive(Clone, Store)]
struct Counter {
    value: i32,
}

#[component]
fn App(name: String) -> Element {
    let count = use_signal(|| 0);
    rsx! { div { "{name}: {count}" } }
}

#[test]
fn prelude_supports_components_and_store_derives() {
    assert_eq!(Counter { value: 1 }.value, 1);
    let _ = App;
}
