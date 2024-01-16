#![allow(non_snake_case)]

//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    render! {
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

type SthElse<T> = Option<T>;

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

fn Button(props: ButtonProps) -> Element {
    render! {
        button {
            "{props.a} | "
            "{props.b:?} | "
            "{props.c:?} | "
            "{props.d:?} | "
            "{props.e:?}"
        }
    }
}
