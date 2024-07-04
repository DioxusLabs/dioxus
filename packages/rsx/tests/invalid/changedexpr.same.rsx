use dioxus::prelude::*;

/// some comment!!!!!
pub fn CoolChild() -> Element {
    let a = 123;

    rsx! {
        div {
            {some_expr()}
        }
    }
}
