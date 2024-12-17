//! A simple example on how to use assets loading from the filesystem.
//!
//! Dioxus provides the asset!() macro which is a convenient way to load assets from the filesystem.
//! This ensures the asset makes it into the bundle through dependencies and is accessible in environments
//! like web and android where assets are lazily loaded using platform-specific APIs.

use dioxus::prelude::*;

static ASSET_PATH: Asset = asset!("/examples/assets/logo.png");

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let a = asset!("/../ecosystem-dioxus/docsite/assets/06assets/355903878-ebcb5872-acf7-4e29-8acb-5b183b0617ca.png");
    let b = asset!("");
    rsx! {
        div {
            h1 { "This should show an image:" }
            img { src: ASSET_PATH }
        }
    }
}
