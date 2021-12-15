#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

fn main() {
    let _ = VirtualDom::new(App);
}

fn App(cx: Scope<()>) -> Element {
    //
    cx.render(rsx!(
        div {
            Child {}
        }
    ))
}

fn Child(cx: Scope<()>) -> Element {
    //
    cx.render(rsx!(
        div {
            h1 {
                "Hello, World!"
            }
        }
    ))
}
