//! XSS Safety
//!
//! This example proves that Dioxus is safe from XSS attacks.

use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let (contents, set_contents) = use_state(&cx, || {
        String::from("<script>alert(\"hello world\")</script>")
    });

    cx.render(rsx! {
        div {
            h1 {"Dioxus is XSS-Safe"}
            h3 { "{contents}" }
            input {
                value: "{contents}",
                r#type: "text",
                oninput: move |e| set_contents(e.value.clone()),
            }
        }
    })
}
