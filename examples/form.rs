//! Forms
//!
//! Dioxus forms deviate slightly from html, automatically returning all named inputs
//! in the "values" field.

use dioxus::prelude::*;
use std::collections::HashMap;

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut values = use_signal(|| HashMap::new());
    rsx! {
        div {
            h1 { "Form" }
            form {
                oninput: move |ev| values.set(ev.values()),
                input {
                    r#type: "text",
                    name: "username",
                    oninput: move |ev| values.set(ev.values())
                }
                input { r#type: "text", name: "full-name" }
                input { r#type: "password", name: "password" }
                input { r#type: "radio", name: "color", value: "red" }
                input { r#type: "radio", name: "color", value: "blue" }
                button { r#type: "submit", value: "Submit", "Submit the form" }
            }
        }
        div {
            h1 { "Oninput Values" }
            "{values:#?}"
        }
    }
}
