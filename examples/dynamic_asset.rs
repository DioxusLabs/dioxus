//! This example shows how to load in custom assets with the use_asset_handler hook.
//!
//! This hook is currently only available on desktop and allows you to intercept any request made by the webview
//! and respond with your own data. You could use this to load in custom videos, streams, stylesheets, images,
//! or any asset that isn't known at compile time.

use dioxus::desktop::{use_asset_handler, wry::http::Response};
use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    use_asset_handler("logos", |request, response| {
        // We get the original path - make sure you handle that!
        if request.uri().path() != "/logos/logo.png" {
            return;
        }

        response.respond(Response::new(include_bytes!("./assets/logo.png").to_vec()));
    });

    rsx! {
        style { {include_str!("./assets/custom_assets.css")} }
        h1 { "Dynamic Assets" }
        img { src: "/logos/logo.png" }
    }
}
