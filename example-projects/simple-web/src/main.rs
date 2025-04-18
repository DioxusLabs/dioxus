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
        h1 { "Hot patch serverfns!!!!!" }
        EvalIt {}
    }
}

fn EvalIt() -> Element {
    rsx! {
        button {
            onclick: move |_| {
                _ = dioxus::document::eval("window.document.body.style.backgroundColor = 'green';");
            },
            "eval!"
        }
    }
}
