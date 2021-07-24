//! Example: Webview Renderer
//! -------------------------
//!
//! This example shows how to use the dioxus_desktop crate to build a basic desktop application.
//!
//! Under the hood, the dioxus_desktop crate bridges a native Dioxus VirtualDom with a custom prebuit application running
//! in the webview runtime. Custom handlers are provided for the webview instance to consume patches and emit user events
//! into the native VDom instance.
//!
//! Currently, NodeRefs won't work properly, but all other event functionality will.

use dioxus::prelude::*;

fn main() {
    env_logger::init();
    dioxus::desktop::launch(App, |c| c);
}

static App: FC<()> = |cx| {
    let mut count = use_state(cx, || 0);

    cx.render(rsx! {
        div {
            h1 { "Hifive counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
        }
    })
};
