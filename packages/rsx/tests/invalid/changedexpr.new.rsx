use dioxus::prelude::*;

/// some comment
pub fn CoolChild() -> Element {
    rsx! {
        div {
            {some_expr()}
        }
    }
}
