use dioxus::prelude::*;

pub fn CoolChild() -> Element {
    rsx! {
        for items in vec![1, 2, 3] {
            div { "asasddasdasd" }
        }
    }
}
