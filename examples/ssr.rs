//! Example: SSR
//!
//! This example shows how we can render the Dioxus Virtualdom using SSR.

use dioxus::prelude::*;

fn main() {
    // We can render VirtualDoms
    let mut vdom = VirtualDom::prebuilt(app);
    println!("{}", dioxus_ssr::render(&vdom));

    // Or we can render render! calls themselves
    println!(
        "{}",
        dioxus_ssr::render_element(render! {
            div {
                h1 { "Hello, world!" }
            }
        })
    );

    // We can configure the SSR rendering to add ids for rehydration
    println!("{}", dioxus_ssr::pre_render(&vdom));

    // We can render to a buf directly too
    let mut file = String::new();
    let mut renderer = dioxus_ssr::Renderer::default();
    renderer.render_to(&mut file, &vdom).unwrap();
    println!("{file}");
}

fn app() -> Element {
    render!(
        div {
            h1 { "Title" }
            p { "Body" }
        }
    )
}
