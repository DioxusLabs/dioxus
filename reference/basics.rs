//! Example: The basics of Dioxus
//! ----------------------------
//!
//! This small example covers some of the basics of Dioxus including
//! - Components
//! - Props
//! - Children
//! - the rsx! macro

use dioxus::prelude::*;

pub static Example: FC<()> = |(cx, props)| {
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

static Greeting: FC<GreetingProps> = |(cx, props)| {
    cx.render(rsx! {
        div {
            h1 { "Hello, {props.name}!" }
            p { "Welcome to the Diouxs framework" }
            br {}
            {cx.children()}
        }
    })
};
