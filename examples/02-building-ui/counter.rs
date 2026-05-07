//! The classic counter — the "hello world" of reactive UI.
//!
//! `use_signal` creates a reactive value. Whenever the signal changes, any part of the UI
//! that reads it will re-render automatically. Signals are copy-cheap, so you can freely
//! move them into event handlers and closures.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        ul { display: "flex", flex_direction: "column",
            a { href: "https://google.com", "goo1" }
            a { href: "google.com", "goo2" }
            a { href: "/foo", "foo1" }
            a { href: "foo", "foo2" }
            iframe {
                width: "560",
                height: "315",
                src: "https://www.youtube.com/embed/mPHNIsDsJio?si=4VjFaOnWIflusRjA",
                title: "YouTube video player",
                "frameborder": "0",
                allow: "accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share",
                // referrerpolicy: "strict-origin-when-cross-origin",
                allowfullscreen: true,
            }
        }
    }
}
