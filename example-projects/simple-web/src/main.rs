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
        h3 { "Set your favorite color" }
        button { onclick: move |_| t += 1, "Click me: {t}" }
        div {
            EvalIt { color: "white" }
            EvalIt { color: "red" }
            EvalIt { color: "blue" }
            EvalIt { color: "green" }
            EvalIt { color: "magenta" }
            EvalIt { color: "orange" }
        }
    }
}

#[component]
fn EvalIt(color: String) -> Element {
    rsx! {
        div {
            button {
                onclick: move |_| {
                    _ = dioxus::document::eval(&format!("window.document.body.style.backgroundColor = '{color}';"));
                },
                "eval -> {color}"
            }
        }
    }
}
