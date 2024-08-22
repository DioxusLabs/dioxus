//! A simple example on how to use assets loading from the filesystem.
//!
//! Dioxus provides an asset!() macro which properly handles asset loading and bundling for you.
//! For bundling, asset!() must be paired with a tool that handles mangansis-link sections. The dioxus-cli
//! handles this for you, but this means you can't just simply `cargo build --release` to build and
//! distribute your app.
//!
//! You can run this example with `cargo run --example assets` or `dx serve --example assets`.
//! When manganis is not active, the asset!() macro will fallback to the path of the asset on
//! your filesystem.

use dioxus::prelude::*;

/// asset!() will mark this asset as a dependency of the app without actually including it in the
/// generated code. This is better than include_str!() or include_bytes!() since it works
/// for web apps as well as native and mobile apps.
///
/// When used with web apps, manganis will detect the import of the image, optimize it, and put it
/// in the output dist folder in the right location, ensuring no two images have the same name.
static ASSET_PATH: ImageAsset = asset!("/examples/assets/logo.png".image().format(ImageType::Avif));

fn main() {
    launch(app);
}

fn app() -> Element {
    rsx! {
        div {
            h1 { "This should show an image:" }
            img { src: ASSET_PATH }

            // keep support for these too
            img {
                src: "/Users/jonkelley/Development/dioxus/examples/assets/logo.png"
            }
            img {
                src: "/examples/assets/logo.png"
            }
            img {
                src: "examples/assets/logo.png"
            }
        }
    }
}
