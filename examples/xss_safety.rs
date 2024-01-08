//! XSS Safety
//!
//! This example proves that Dioxus is safe from XSS attacks.

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let contents = use_state(cx, || {
        String::from("<script>alert(\"hello world\")</script>")
    });

    cx.render(rsx! {
        div {
            h1 {"Dioxus is XSS-Safe"}
            h3 { "{contents}" }
            input {
                value: "{contents}",
                r#type: "text",
                oninput: move |e| contents.set(e.value()),
            }
        }
    })
}
