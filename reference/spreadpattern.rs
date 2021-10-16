//! Example: Spread pattern for Components
//! --------------------------------------
//!
//! Dioxus supports  the "spread" pattern for manually building a components properties. This is useful when props
//! are passed down from a parent, or it's more ergonomic to construct props from outside the rsx! macro.
//!
//! To use the spread pattern, simply pass ".." followed by a Rust epxression. This pattern also supports overriding
//! values, using the manual props as the base and then modifying fields specified with non-spread attributes.

use dioxus::prelude::*;

pub static Example: FC<()> = |(cx, props)| {
    let props = MyProps {
        count: 0,
        live: true,
        name: "Dioxus",
    };
    cx.render(rsx! {
        Example1 { ..props, count: 10, div {"child"} }
    })
};

#[derive(PartialEq, Props)]
pub struct MyProps {
    count: u32,
    live: bool,
    name: &'static str,
}

pub static Example1: FC<MyProps> = |cx, MyProps { count, live, name }| {
    cx.render(rsx! {
        div {
            h1 { "Hello, {name}"}
            h3 {"Are we alive? {live}"}
            p {"Count is {count}"}
            { cx.children() }
        }
    })
};
