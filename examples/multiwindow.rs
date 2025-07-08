//! Multiwindow example
//!
//! This example shows how to implement a simple multiwindow application using dioxus.
//! This works by spawning a new window when the user clicks a button. We have to build a new virtualdom which has its
//! own context, root elements, etc.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let onclick = move |_| {
        dioxus::desktop::window().new_window(VirtualDom::new(popup), Default::default());
    };

    rsx! {
        button { onclick, "New Window" }
    }
}

fn popup() -> Element {
    let mut count = use_signal(|| 0);
    rsx! {
        div {
            h1 { "Popup Window" }
            p { "Count: {count}" }
            button { onclick: move |_| count += 1, "Increment" }
        }
    }
}
