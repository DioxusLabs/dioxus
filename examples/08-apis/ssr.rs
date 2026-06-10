//! Example: SSR
//!
//! This example shows how we can render the Dioxus Virtualdom using SSR.
//! Dioxus' SSR output is hydratable without inserting marker attributes, as long as the server
//! and client build matching VirtualDOMs.

use dioxus::prelude::*;

fn main() {
    // We can render VirtualDoms
    let vdom = VirtualDom::prebuilt(app);
    println!("{}", dioxus_ssr::render(&vdom));

    // Or we can render rsx! calls themselves
    println!(
        "{}",
        dioxus_ssr::render_element(rsx! {
            div {
                h1 { "Hello, world!" }
            }
        })
    );

    // Hydration uses the same clean SSR output.
    println!("{}", dioxus_ssr::render(&vdom));

    // We can render to a buf directly too
    let mut file = String::new();
    let mut renderer = dioxus_ssr::Renderer::default();
    renderer.render_to(&mut file, &vdom).unwrap();
    println!("{file}");
}

fn app() -> Element {
    rsx!(
        div {
            h1 { "Title" }
            p { "Body" }
        }
    )
}
