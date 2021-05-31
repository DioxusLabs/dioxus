use dioxus_core::prelude::*;
use dioxus_core_macro::format_args_f;

fn main() {
    let num = 123;
    let b = Bump::new();

    let g = rsx! {
        div {
            "abc {num}"
            div {
                "asd"
            }
        }
    };
}
