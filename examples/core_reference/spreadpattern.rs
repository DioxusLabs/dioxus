//! Example: Spread pattern for Components
//! --------------------------------------
//!
//! Dioxus supports  the "spread" pattern for manually building a component's properties. This is useful when props
//! are passed down from a parent, or it's more ergonomic to construct props from outside the rsx! macro.
//!
//! To use the spread pattern, simply pass ".." followed by a Rust expression. This pattern also supports overriding
//! values, using the manual props as the base and then modifying fields specified with non-spread attributes.

use dioxus::prelude::*;

pub fn Example(cx: Scope) -> Element {
    let props = MyProps {
        count: 0,
        live: true,
        name: "Dioxus",
    };
    cx.render(rsx! {
        Example1 { ..props, count: 10, div {"child"} }
    })
}

#[derive(Props)]
pub struct MyProps<'a> {
    count: u32,
    live: bool,
    name: &'static str,
    children: Element<'a>,
}

pub fn Example1<'a>(cx: Scope<'a, MyProps<'a>>) -> Element {
    let MyProps { count, live, name } = cx.props;
    cx.render(rsx! {
        div {
            h1 { "Hello, {name}"}
            h3 {"Are we alive? {live}"}
            p {"Count is {count}"}
            &cx.props.children
        }
    })
}
