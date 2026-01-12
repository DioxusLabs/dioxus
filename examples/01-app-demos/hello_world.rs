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

use dioxus::{desktop::DesktopServiceProxy, prelude::*};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // From a component, get the proxy from context and set the window ID
    let proxy = use_context::<DesktopServiceProxy>();

    // Run a closure synchronously on the main thread
    let title = proxy.run_with_desktop_service(|desktop| desktop.window.title().to_string());

    rsx! {
        div { "The title is {title}" }
    }
}
