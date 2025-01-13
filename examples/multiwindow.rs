//! Multiwindow example
//!
//! This example shows how to implement a simple multiwindow application using dioxus.
//! This works by spawning a new window when the user clicks a button. We have to build a new virtualdom which has its
//! own context, root elements, etc.

use dioxus::prelude::*;
use dioxus::{desktop::Config, desktop::WindowCloseBehaviour};

fn main() {
    dioxus::LaunchBuilder::desktop()
        // We can choose the close behavior of the last window to hide. See WindowCloseBehaviour for more options.
        .with_cfg(Config::new().with_close_behaviour(WindowCloseBehaviour::LastWindowHides))
        .launch(app);
}

fn app() -> Element {
    let onclick = move |_| {
        let dom = VirtualDom::new(popup);
        dioxus::desktop::window().new_window(dom, Default::default());
    };

    rsx! {
        button { onclick, "New Window" }
    }
}

fn popup() -> Element {
    rsx! {
        div { "This is a popup window!" }
    }
}
