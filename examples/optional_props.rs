#![allow(non_snake_case)]

//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    rsx! {
        Button {
            a: "asd".to_string(),
            c: "asd".to_string(),
            d: Some("asd".to_string()),
            e: Some("asd".to_string()),
        }
        Button {
            a: "asd".to_string(),
            b: "asd".to_string(),
            c: "asd".to_string(),
            d: Some("asd".to_string()),
            e: "asd".to_string(),
        }
        Button {
            a: "asd".to_string(),
            c: "asd".to_string(),
            d: Some("asd".to_string()),
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct ButtonProps {
    a: String,

    #[props(default)]
    b: String,

    c: Option<String>,

    #[props(!optional)]
    d: Option<String>,

    #[props(optional)]
    e: SthElse<String>,
}

type SthElse<T> = Option<T>;

fn Button(props: ButtonProps) -> Element {
    rsx! {
        button {
            "{props.a} | "
            "{props.b:?} | "
            "{props.c:?} | "
            "{props.d:?} | "
            "{props.e:?}"
        }
    }
}
