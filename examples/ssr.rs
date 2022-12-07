//! Example: SSR
//!
//! This example shows how we can render the Dioxus Virtualdom using SSR.

use std::fmt::Write;

use dioxus::prelude::*;
use dioxus_ssr::config::Config;

fn main() {
    // We can render VirtualDoms
    let mut vdom = VirtualDom::new(app);
    let _ = vdom.rebuild();
    println!("{}", dioxus_ssr::render_vdom(&vdom));

    // Or we can render rsx! calls themselves
    println!(
        "{}",
        dioxus_ssr::render_lazy(rsx! {
            div {
                h1 { "Hello, world!" }
            }
        })
    );

    // We can configure the SSR rendering to add ids for rehydration
    println!(
        "{}",
        dioxus_ssr::render_vdom_cfg(&vdom, Config::default().pre_render(true))
    );

    // We can even render as a writer
    let mut file = String::new();
    let _ = file.write_fmt(format_args!(
        "{}",
        dioxus_ssr::SsrRender::default().render_vdom(&vdom)
    ));
    println!("{}", file);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!(
        div {
            h1 { "Title" }
            p { "Body" }
        }
    ))
}
