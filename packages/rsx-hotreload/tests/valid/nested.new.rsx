use dioxus::prelude::*;

pub fn CoolChild() -> Element {
    rsx! {
        ForLoop {
            div { "123" }
            div { "asasddasdasd" }
        }
    }
}
