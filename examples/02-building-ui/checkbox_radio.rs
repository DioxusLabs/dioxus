//! Checkboxes and radio groups.
//!
//! Checkboxes expose their checked state through the FormEvent — read `evt.checked()`.
//! Radio groups share a `name` attribute so only one option can be selected at a time;
//! the selected value comes through `evt.value()`.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut subscribed = use_signal(|| false);
    let mut color = use_signal(|| "red".to_string());

    rsx! {
        h2 { "Checkbox" }
        label {
            input {
                r#type: "checkbox",
                checked: subscribed(),
                oninput: move |evt| subscribed.set(evt.checked()),
            }
            " Subscribe to updates"
        }
        p { "Subscribed: {subscribed}" }

        h2 { "Radio group" }
        // Radio inputs sharing a `name` are mutually exclusive
        for value in ["red", "green", "blue"] {
            label {
                input {
                    r#type: "radio",
                    name: "color",
                    value,
                    checked: color() == value,
                    oninput: move |evt| color.set(evt.value()),
                }
                " {value}"
            }
        }
        p { color: "{color}", "Chosen color: {color}" }
    }
}
