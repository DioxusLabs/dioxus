//! Example: SSR
//!
//! This example shows how we can render the Dioxus Virtualdom using SSR.

use std::fmt::Write;

use dioxus::prelude::*;

fn main() {
    // We can render VirtualDoms
    let mut vdom = VirtualDom::new(app);
    let _ = vdom.rebuild();
    println!("{}", dioxus::ssr::render_vdom(&vdom));

    // Or we can render rsx! calls themselves
    println!(
        "{}",
        dioxus::ssr::render_lazy(rsx! {
            div {
                h1 { "Hello, world!" }
            }
        })
    );

    // render_lazy can also take components - but it won't be able to process futures
    println!(
        "{}",
        dioxus::ssr::render_lazy(rsx! {
            div {
                Child {}
            }
        })
    );

    // We can configure the SSR rendering to add ids for rehydration
    println!(
        "{}",
        dioxus::ssr::render_vdom_cfg(&vdom, |c| c.pre_render(true))
    );

    // We can even render as a writer
    let mut file = String::new();
    let _ = file.write_fmt(format_args!(
        "{}",
        dioxus::ssr::TextRenderer::from_vdom(&vdom, Default::default())
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

#[allow(non_snake_case)]
fn Child(_cx: Scope) -> Element {
    None
}
