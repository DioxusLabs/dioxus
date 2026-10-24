//! Linking a CSS stylesheet.
//!
//! `Stylesheet` renders a `<link rel="stylesheet">` into the document head. Pairing it with
//! the `asset!` macro is the most common way to style a Dioxus app: the CSS file is bundled,
//! fingerprinted for cache-busting, and can be hot-reloaded during development.

use dioxus::prelude::*;

const STYLE: Asset = asset!("/examples/assets/stylesheet.css");

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        // Placing the Stylesheet anywhere in the tree is fine — it's collected into <head>
        Stylesheet { href: STYLE }

        h1 { "Styled with a CSS file" }
        p { "All styling comes from stylesheet.css, loaded via the asset! macro." }
        button { "A styled button" }
    }
}
