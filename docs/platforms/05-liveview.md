## Liveview

With the Context, Subscription, and Asynchronous APIs, we've built Dioxus Liveview: a coupling of frontend and backend to deliver user experiences that do not require dedicated API development. Instead of building and maintaining frontend-specific API endpoints, components can directly access databases, server caches, and other services directly from the component.

These set of features are still experimental. Currently, we're still working on making these components more ergonomic

```rust
fn live_component(cx: &Context<()>) -> DomTree {
    use_live_component(
        cx,
        // Rendered via the client
        #[cfg(target_arch = "wasm32")]
        || html! { <div> {"Loading data from server..."} </div> },

        // Renderered on the server
        #[cfg(not(target_arch = "wasm32"))]
        || html! { <div> {"Server Data Loaded!"} </div> },
    )
}
```
