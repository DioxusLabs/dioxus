//! Static generation lets you pre-render your entire app to static files and then hydrate it on the client.
use dioxus::prelude::*;

// Generate all routes and output them to the static path
fn main() {
    launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    }
}
