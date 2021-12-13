#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

fn main() {
    let _ = VirtualDom::new(App);
}

fn App(cx: Context, _props: &()) -> Element {
    //
    cx.render(rsx!(
        div {
            Child {}
        }
    ))
}

struct ChildProps<'a> {
    click_handler: EventHandler<'a>,
}

fn Child(cx: Context, _props: &()) -> Element {
    //
    cx.render(rsx!(
        div {
            h1 {
                "Hello, World!"
            }
        }
    ))
}
