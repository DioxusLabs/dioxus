//! Setting a favicon.
//!
//! Use `document::Link` with `rel: "icon"` to point the browser at a favicon. Combined with
//! `asset!`, the favicon is bundled into the app and served with the same path-mangling
//! and cache-busting as any other asset.

use dioxus::prelude::*;

const FAVICON: Asset = asset!("/examples/assets/logo.png");

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        // Link is a generic head element — for a favicon set rel="icon"
        document::Link { rel: "icon", href: FAVICON }

        h1 { "Check the browser tab!" }
        p { "The favicon is loaded from examples/assets/logo.png." }
    }
}
