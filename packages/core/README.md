# Dioxus-core

This is the core crate for the Dioxus Virtual DOM. This README will focus on the technical design and layout of this Virtual DOM implementation. If you want to read more about using Dioxus, then check out the Dioxus crate, documentation, and website.

We reserve the "dioxus" name and aggregate all the various renderers under it. If you want just a single dioxus renderer, then chose from "dioxus-web", "dioxus-desktop", etc.

## Internals
Dioxus-core builds off the many frameworks that came before it. Notably, Dioxus borrows these concepts:

- React: hooks, concurrency, suspense
- Dodrio: bump allocation, double buffering, and source code for nodes + NodeBuilder
- Percy: html! macro architecture, platform-agnostic edits
- Yew: passion and inspiration ❤️

## Goals

We have big goals for Dioxus. The final implementation must:

- Be **fast**. Allocators are typically slow in WASM/Rust, so we should have a smart way of allocating.
- Be extremely memory efficient. Servers should handle tens of thousands of simultaneous VDoms with no problem.
- Be concurrent. Components should be able to pause rendering using a threading mechanism.
- Be "remote". Edit lists should be separate from the Renderer implementation.
- Support SSR. VNodes should render to a string that can be served via a web server.
- Be "live". Components should be able to be both server rendered and client rendered without needing frontend APIs.
- Be modular. Components and hooks should be work anywhere without worrying about target platform.

## Optimizations

- Support a pluggable allocation strategy that makes VNode creation **very** fast
- Support lazy DomTrees (ie DomTrees that are not actually created when the html! macro is used)
- Support advanced diffing strategies (patience, Myers, etc)

## Design Quirks

- Use of "Context" as a way of mitigating threading issues and the borrow checker. (JS relies on globals)
- html! is lazy - needs to be used with a partner function to actually allocate the html. (Good be a good thing or a bad thing)

```rust
let text = TextRenderer::render(html! {<div>"hello world"</div>});
// <div>hello world</div>
```

```rust

fn main() {
    tide::new()
        .get("blah", serve_app("../"))
        .get("blah", ws_handler(serve_app))
}


fn serve_app(ctx: &Context<()>) -> VNode {
    let livecontext = LiveContext::new()
        .with_handler("graph", graph_component)
        .with_handler("graph", graph_component)
        .with_handler("graph", graph_component)
        .with_handler("graph", graph_component)
        .with_handler("graph", graph_component)
        .with_handler("graph", graph_component)
        .build();

    ctx.render(html! {
        <LiveContext ctx={livecontext}>
            <App />
        </ LiveContext>
    })
}

```
