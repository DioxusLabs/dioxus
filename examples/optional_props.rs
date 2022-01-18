#![allow(non_snake_case)]

//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Button {
            a: "asd".to_string(),
            c: Some("asd".to_string()),
            d: "asd".to_string(),
            e: "asd".to_string(),
        }
    })
}

#[derive(Props, PartialEq)]
struct ButtonProps {
    a: String,

    #[props(default)]
    b: Option<String>,

    #[props(default)]
    c: Option<String>,

    #[props(default, strip_option)]
    d: Option<String>,

    #[props(optional)]
    e: Option<String>,
}

fn Button(cx: Scope<ButtonProps>) -> Element {
    cx.render(rsx! {
        button {
            "{cx.props.a}"
            "{cx.props.b:?}"
            "{cx.props.c:?}"
            "{cx.props.d:?}"
            "{cx.props.e:?}"
        }
    })
}
