//! Demonstrates CSS files that reference other assets via `url()` and `@import`.
//!
//! The CSS file imports another CSS file that uses `url("logo.png")`.
//! Manganis discovers these references, registers them as assets, and rewrites
//! the URLs to use the hashed bundled paths.
//!
//! Run with: `dx serve --example css_references`
//!
//! After building, check the output assets directory — you should see:
//! - css_references-dxh<hash>.css  (with rewritten @import URL)
//! - css_references_base-dxh<hash>.css  (with rewritten url() to logo)
//! - logo-dxh<hash>.png  (discovered from the CSS, not declared in Rust)

use dioxus::prelude::*;

static STYLE: Asset = asset!("/examples/assets/css_references.css");

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: STYLE }
        div {
            h1 { "CSS Asset References" }
            p { "The background image below is loaded via a CSS url() reference." }
            p { "It was discovered automatically from the CSS — no asset!() needed in Rust." }
            div { class: "hero" }
        }
    }
}
