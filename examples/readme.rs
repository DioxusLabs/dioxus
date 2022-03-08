//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

use dioxus_html::element_builder::IntoAttributeValue;
use dioxus_html::elements::h1;
use dioxus_html::HtmlElement;

fn app(cx: Scope) -> Element {
    let count = use_state(&cx, || 0);

    cx.render2([
        h1(&cx).text("High-Five counter: {count}"),
        button(&cx).text("Up High").onclick(move |_| count += 1),
        button(&cx).text("Down Low").onclick(move |_| count -= 1),
    ])
}
