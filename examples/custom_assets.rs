//! A simple example on how to use assets loading from the filesystem.
//!
//! If the feature "collect-assets" is enabled, the assets will be collected via the dioxus CLI and embedded into the
//! final bundle. This lets you do various useful things like minify, compress, and optimize your assets.
//!
//! We can still use assets without the CLI middleware, but generally larger apps will benefit from it.

use dioxus::prelude::*;

#[cfg(not(feature = "collect-assets"))]
static ASSET_PATH: &str = "examples/assets/logo.png";

#[cfg(feature = "collect-assets")]
static ASSET_PATH: &str =
    manganis::mg!(image("examples/assets/logo.png").format(ImageType::Avif)).path();

fn main() {
    launch(app);
}

fn app() -> Element {
    rsx! {
        div {
            h1 { "This should show an image:" }
            img { src: ASSET_PATH.to_string() }
        }
    }
}
