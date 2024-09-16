//! Example: real-world usage of hydration
//! ------------------------------------
//!
//! This example shows how to pre-render a page using dioxus SSR and then how to rehydrate it on the client side.
//!
//! To accomplish hydration on the web, you'll want to set up a slightly more sophisticated build & bundle strategy. In
//! the official docs, we have a guide for using DioxusStudio as a build tool with pre-rendering and hydration.
//!
//! In this example, we pre-render the page to HTML and then pass it into the desktop configuration. This serves as a
//! proof-of-concept for the hydration feature, but you'll probably only want to use hydration for the web.

use dioxus::desktop::Config;
use dioxus::prelude::*;

fn main() {
    LaunchBuilder::desktop()
        .with_cfg(Config::new().with_prerendered({
            // We build the dom a first time, then pre-render it to HTML
            let pre_rendered_dom = VirtualDom::prebuilt(app);

            // We then launch the app with the pre-rendered HTML
            dioxus_ssr::pre_render(&pre_rendered_dom)
        }))
        .launch(app)
}

fn app() -> Element {
    let mut val = use_signal(|| 0);

    rsx! {
        div {
            h1 { "hello world. Count: {val}" }
            button { onclick: move |_| val += 1, "click to increment" }
        }
    }
}
