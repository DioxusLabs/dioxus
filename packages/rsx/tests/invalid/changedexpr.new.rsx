use dioxus::prelude::*;

pub fn CoolChild() -> Element {
    rsx! {
        div {
            {some_expr()}
        }
    }
}
