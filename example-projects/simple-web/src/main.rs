//! Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut t = use_signal(|| 0);
    rsx! {
        h1 { "Hot patch serverfns!" }
        button {
            onclick: move |_| {
                t += 1;
            },
            "Say hi!"
        }
        "{t}"
    }
}
