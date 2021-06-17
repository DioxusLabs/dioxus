use dioxus_core as dioxus;
use dioxus_core::events::on::*;
use dioxus_core::prelude::*;

fn main() {}

fn autocomplete() {
    let handler = move |evt| {
        let r = evt.alt_key();
        if evt.alt_key() {}
    };

    let g = rsx! {
        button {
            button {
                onclick: {handler}
            }
        }

    };
}
