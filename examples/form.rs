//! Forms
//!
//! Dioxus forms deviate slightly from html, automatically returning all named inputs
//! in the "values" field.

use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    rsx! {
        div {
            h1 { "Form" }
            form {
                onsubmit: move |ev| println!("Submitted {:?}", ev.values()),
                oninput: move |ev| println!("Input {:?}", ev.values()),
                input { r#type: "text", name: "username" }
                input { r#type: "text", name: "full-name" }
                input { r#type: "password", name: "password" }
                input { r#type: "radio", name: "color", value: "red" }
                input { r#type: "radio", name: "color", value: "blue" }
                button { r#type: "submit", value: "Submit", "Submit the form" }
            }
        }
    }
}
