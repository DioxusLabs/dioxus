//! Controlled text inputs.
//!
//! To read text from an input, attach an `oninput` handler and call `evt.value()` to get the
//! current value as a `String`. Binding the `value:` attribute to a signal makes the input
//! "controlled" — the signal is the single source of truth, so clearing it clears the input.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut name = use_signal(String::new);
    let mut message = use_signal(|| "Type something below.".to_string());

    rsx! {
        h1 { "Hello, {name}!" }

        input {
            r#type: "text",
            placeholder: "Your name",
            // Binding `value` to a signal makes this a controlled input
            value: "{name}",
            oninput: move |evt| name.set(evt.value()),
        }

        button {
            onclick: move |_| name.set(String::new()),
            "Clear"
        }

        hr {}

        p { "{message}" }
        textarea {
            rows: 3,
            cols: 40,
            value: "{message}",
            oninput: move |evt| message.set(evt.value()),
        }
    }
}
