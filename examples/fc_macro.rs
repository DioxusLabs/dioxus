use dioxus::prelude::*;
use dioxus_ssr::TextRenderer;

// todo @Jon, support components in the html! macro
// let renderer = TextRenderer::new(|_| html! {<Example name="world"/>});
fn main() {
    let renderer = TextRenderer::<()>::new(|_| html! {<div> "Hello world" </div>});
    let output = renderer.render();
}

/// An example component that demonstrates how to use the functional_component macro
/// This macro makes writing functional components elegant, similar to how Rocket parses URIs.
///
/// You don't actually *need* this macro to be productive, but it makes life easier, and components cleaner.
/// This approach also integrates well with tools like Rust-Analyzer
#[fc]
fn example(ctx: &Context, name: String) -> VNode {
    html! { <div> "Hello, {name}!" </div> }
}
