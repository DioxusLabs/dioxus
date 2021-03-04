//! Example: Webview Renderer
//! -------------------------
//!
//! This example shows how to use the dioxus_webview crate to build a basic desktop application.
//!
//! Under the hood, the dioxus_webview crate bridges a native Dioxus VirtualDom with a custom prebuit application running
//! in the webview runtime. Custom handlers are provided for the webview instance to consume patches and emit user events
//! into the native VDom instance.

use dioxus::prelude::*;

async fn main() {
    dioxus_webview::launch(|ctx| {
        let (count, set_count) = use_state(ctx, || 0);

        ctx.render(rsx! {
            div {
                h1 { "Dioxus Desktop Demo" }
                p { "Count is {count}" }
                button {
                    "Click to increment"
                    onclick: |_| set_count(count + 1)
                }
            }
        })
    }).await;
}
