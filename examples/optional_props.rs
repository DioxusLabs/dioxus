#![allow(non_snake_case)]

//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
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
    })
}

type SthElse<T> = Option<T>;

#[derive(Props, PartialEq)]
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

fn Button(cx: Scope<ButtonProps>) -> Element {
    cx.render(rsx! {
        button {
            "{cx.props.a} | "
            "{cx.props.b:?} | "
            "{cx.props.c:?} | "
            "{cx.props.d:?} | "
            "{cx.props.e:?}"
        }
    })
}
