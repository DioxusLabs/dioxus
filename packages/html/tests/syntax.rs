use std::fmt::Arguments;

use dioxus_core::prelude::*;
use dioxus_html::builder::*;

use dioxus_html::{element_builder::AnyBuilder, HtmlElement};

fn Demo(cx: Scope) -> Element {
    let count = &*cx.use_hook(|_| 0);

    cx.render([
        h1(&cx).text(format_args!("Count: {count}")),
        button(&cx).text("Up High").onclick(move |_| count += 1),
        button(&cx).text("Down Low").onclick(move |_| count -= 1),
    ])
}
