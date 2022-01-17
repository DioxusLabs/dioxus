//! This example shows that autocomplete works in RSX

use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            onclick: move |_| {
            }
            // class: "asd",
            // style {
            //     media: "Ad",
            // }
            // div {

            // }
            // {
            //     let t = String::new();
            //     t.
            // }
        }
    })
}
