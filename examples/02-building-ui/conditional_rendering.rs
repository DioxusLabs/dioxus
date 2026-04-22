//! Conditional rendering.
//!
//! `if`, `else`, and `match` all work directly inside `rsx!`. You can also return different
//! `rsx!` blocks early from a component function. Elements outside of a conditional will
//! render normally — only the branched portion swaps as state changes.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

#[derive(PartialEq, Clone, Copy)]
enum Status {
    Loading,
    Ready,
    Error,
}

fn app() -> Element {
    let mut logged_in = use_signal(|| false);
    let mut status = use_signal(|| Status::Loading);

    rsx! {
        h1 { "Conditional rendering" }

        // `if/else` chains render the matching branch
        if logged_in() {
            p { "Welcome back!" }
        } else {
            p { "Please log in." }
        }
        button { onclick: move |_| logged_in.toggle(), "Toggle login" }

        // `match` works too — each arm returns rsx!
        div {
            match status() {
                Status::Loading => rsx! { p { "⏳ Loading..." } },
                Status::Ready => rsx! { p { "✅ Ready!" } },
                Status::Error => rsx! { p { "❌ Something went wrong." } },
            }
        }
        button { onclick: move |_| status.set(Status::Loading), "Loading" }
        button { onclick: move |_| status.set(Status::Ready), "Ready" }
        button { onclick: move |_| status.set(Status::Error), "Error" }

        // `if` without an `else` is also valid — nothing renders when false
        if logged_in() && status() == Status::Ready {
            p { "You are logged in and the app is ready." }
        }
    }
}
