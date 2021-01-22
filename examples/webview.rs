//! Example: Webview Renderer
//!
//! This example shows how to use the dioxus_webview crate to build a basic desktop application.
//!
//! Under the hood, the dioxus_webview crate bridges a native Dioxus VirtualDom with a custom prebuit application running
//! in the webview runtime. Custom handlers are provided for the webview instance to consume patches and emit user events
//! into the native VDom instance.

use dioxus::prelude::*;

fn main() {
    let app = dioxus_webview::new(|ctx| {
        let (count, set_count) = use_state(ctx, || 0);

        html! {
            <div>
                <h1> "Dioxus Desktop Demo" </h1>
                <p> "Count is {count}"</p>
                <p> "Count is {count}"</p>
                <p> "Data is {data}"</p>
                <button onclick=|_| set_count(count + 1) >
                    "Click to increment"
                </button>
             </div>
        }
    });

    app.launch(());
}
