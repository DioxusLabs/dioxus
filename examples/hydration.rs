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

use dioxus::prelude::*;
use dioxus::ssr;

fn main() {
    let vdom = VirtualDom::new(app);
    let content = ssr::render_vdom_cfg(&vdom, |f| f.pre_render(true));

    dioxus::desktop::launch_cfg(app, |c| c.with_prerendered(content));
}

fn app(cx: Scope) -> Element {
    let val = use_state(&cx, || 0);

    cx.render(rsx! {
        div {
            h1 { "hello world. Count: {val}" }
            button {
                onclick: move |_| *val.make_mut() += 1,
                "click to increment"
            }
        }
    })
}
