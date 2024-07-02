use dioxus::prelude::*;

pub fn CoolChild() -> Element {
    rsx! {
        if cond() {
            div { "123" }
            div { "asasddasdasd" }
        }
    }
}
