//! Forms
//!
//! Dioxus forms deviate slightly from html, automatically returning all named inputs
//! in the "values" field

use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            h1 { "Form" }
            form {
                onsubmit: move |ev| println!("Submitted {:?}", ev.values),
                oninput: move |ev| println!("Input {:?}", ev.values),
                input { r#type: "text", name: "username" }
                input { r#type: "text", name: "full-name" }
                input { r#type: "password", name: "password" }
                button { "Submit the form" }
            }
        }
    })
}
