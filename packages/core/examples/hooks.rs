use dioxus::prelude::*;
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_hooks::use_state;
use dioxus_html as dioxus_elements;

fn main() {}

fn App(cx: Scope<()>) -> Element {
    let color = use_state(&cx, || "white");

    cx.render(rsx!(
        div { onclick: move |_| color.set("red"), "red" }
        div { onclick: move |_| color.set("blue"), "blue" }
    ))
}
