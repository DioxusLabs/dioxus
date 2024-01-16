//! XSS Safety
//!
//! This example proves that Dioxus is safe from XSS attacks.

use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut contents = use_signal(|| String::from("<script>alert(\"hello world\")</script>"));

    rsx! {
        div {
            h1 {"Dioxus is XSS-Safe"}
            h3 { "{contents}" }
            input {
                value: "{contents}",
                r#type: "text",
                oninput: move |e| contents.set(e.value()),
            }
        }
    }
}
