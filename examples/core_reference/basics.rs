//! Example: The basics of Dioxus
//! ----------------------------
//!
//! This small example covers some of the basics of Dioxus including
//! - Components
//! - Props
//! - Children
//! - the rsx! macro

use dioxus::prelude::*;

pub static Example: Component = |cx| {
    cx.render(rsx! {
        div {
            Greeting {
                name: "Dioxus"
                div { "Dioxus is a fun, fast, and portable UI framework for Rust" }
            }
        }
    })
};

#[derive(PartialEq, Props)]
struct GreetingProps {
    name: &'static str,
}

static Greeting: Component<GreetingProps> = |cx| {
    cx.render(rsx! {
        div {
            h1 { "Hello, {cx.props.name}!" }
            p { "Welcome to the Dioxus framework" }
            br {}
            {cx.children()}
        }
    })
};
