//! Example: Spread pattern for Components
//! --------------------------------------
//!
//! Dioxus supports  the "spread" pattern for manually building a components properties. This is useful when props
//! are passed down from a parent, or it's more ergonomic to construct props from outside the rsx! macro.
//!
//! To use the spread pattern, simply pass ".." followed by a Rust epxression. This pattern also supports overriding
//! values, using the manual props as the base and then modifying fields specified with non-spread attributes.

use dioxus::prelude::*;
fn main() {}

static App: FC<()> = |cx| {
    let props = MyProps {
        count: 0,
        live: true,
        name: "Dioxus",
    };
    cx.render(rsx! {
        Example { ..props, count: 10, div {"child"} }
    })
};

#[derive(PartialEq, Props)]
struct MyProps {
    count: u32,
    live: bool,
    name: &'static str,
}

static Example: FC<MyProps> = |cx| {
    cx.render(rsx! {
        div {
            h1 { "Hello, {cx.name}"}
            h3 {"Are we alive? {cx.live}"}
            p {"Count is {cx.count}"}
            { cx.children() }
        }
    })
};
