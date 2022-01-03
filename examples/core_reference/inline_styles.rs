//! Example: Inline Styles
//! ----------------------
//!
//! This example shows how to use inline styles in Dioxus components.
//!
//! Inline styles function very similarly to regular attributes, just grouped together in "style".
//!
//! Inline styles in Dioxus are more performant than React since we're able to cache attributes and compare by pointers.
//! However, it's still not as performant as cascaded styles. Use with care.

use dioxus::prelude::*;

pub fn Example(cx: Scope) -> Element {
    cx.render(rsx! {
        head {
            background_color: "powderblue"
         }
        body {
            h1 {
                color: "blue",
                "This is a heading"
            }
            p {
                color: "red",
                "This is a paragraph"
            }
        }
    })
}
