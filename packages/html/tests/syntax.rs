use dioxus_core::IntoVNode;
use std::fmt::Arguments;

use dioxus_core::prelude::*;
use dioxus_html::builder::*;

use dioxus_html::HtmlElement;

fn Demo(cx: Scope) -> Element {
    let count = &*cx.use_hook(|_| 0);

    cx.render2([
        h1(&cx).text(format_args!("Count: {count}")),
        button(&cx).text("Up High"),
        button(&cx).text("Up High"),
    ])
}
