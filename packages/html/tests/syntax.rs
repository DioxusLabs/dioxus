use dioxus_core::prelude::*;
use dioxus_html::builder::*;

use dioxus_html::{element_builder::AnyBuilder, HtmlElement};

fn Demo(cx: Scope) -> Element {
    div(&cx)
        .hidden(true)
        .dir("asd")
        .contenteditable(true)
        .data("asd")
        .draggable(false)
        .class("asd")
        .classname("job")
        .classname("bob")
        .classname("bob")
        .classname("bob")
        .classname("bob")
        .classname("bob")
        .accesskey("asd")
        .prevent_default("onclick")
        .onclick(move |_| {})
        .children([
            div(&cx).class("asd asd asd asd "),
            div(&cx).class("asd asd asd asd "),
            div(&cx).class("asd asd asd asd "),
            div(&cx).class("asd asd asd asd "),
            h1(&cx).height("150px"),
            fragment(&cx).children([]),
        ])
        .render()
}
