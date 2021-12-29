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

pub static Example: Component = |cx| {
    cx.render(rsx! {
        head {
            style: { background_color: "powderblue" }
         }
        body {
            h1 { style: { color: "blue" }
                "This is a heading"
            }
            p { style: { color: "red" }
                "This is a paragraph"
            }
        }
    })
};

// .... technically the rsx! macro is slightly broken at the moment and allows styles not wrapped in style {}
// I haven't noticed any name collisions yet, and am tentatively leaving this behavior in..
// Don't rely on it.
static Example2: Component = |cx| {
    cx.render(rsx! {
        div { color: "red"
            "hello world!"
        }
    })
};
