//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_hooks::use_state;
use dioxus_html as dioxus_elements;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    dioxus_web::launch(App);
}

static App: Component = |cx| {
    let show = use_state(&cx, || true);

    let inner = match *show {
        true => {
            rsx!( div {
                "hello world"
            })
        }
        false => {
            rsx!( div {
                // h1 {
                    "bello world"
                // }
            })
        }
    };

    rsx!(cx, div {
        button {
            "toggle"
            onclick: move |_| {
                let cur = *show;
                show.set(!cur);
            }
        }
        {inner}
    })
};
