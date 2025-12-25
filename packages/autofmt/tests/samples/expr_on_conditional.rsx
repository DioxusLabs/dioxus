//! A comment about this file

use dioxus::prelude::*;

#[component]
pub fn Sample() -> Element {
    let is_active = use_signal(|| false);

    rsx! {
        div { class: if is_active() { "active" } else { "inactive" },
            div { class: if is_active2() { "a" } else { "b" } }
        }
    }
}
