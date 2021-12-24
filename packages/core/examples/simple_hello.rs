use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

// very tiny hello world
fn main() {
    dioxus::VirtualDom::new(|cx| rsx!(cx, "hello world"));
}
