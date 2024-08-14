//! The simplest example of a Dioxus app.
//!
//! In this example we:
//! - import a number of important items from the prelude (launch, Element, rsx, div, etc.)
//! - define a main function that calls the launch function with our app function
//! - define an app function that returns a div element with the text "Hello, world!"
//!
//! The `launch` function is the entry point for all Dioxus apps. It takes a function that returns an Element. This function
//! calls "launch" on the currently-configured renderer you have. So if the `web` feature is enabled, it will launch a web
//! app, and if the `desktop` feature is enabled, it will launch a desktop app.

use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    rsx! {
        div {
            MyNav {} // 1 -> 3
            MyOutlet {}
        }
    }
}

fn MyNav() -> Element {
    rsx!(
        div { "trailing nav" }
        div { "whhhhh"}
        div { "bhhhh" }
    )
}

fn MyOutlet() -> Element {
    rsx!(
        div { "homepage 1" }
    )
}

/*
root
root app
root app search
root app search trailing
root app
root app homepage
root app
root app footer
root
*/
