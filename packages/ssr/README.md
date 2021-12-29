# Dioxus SSR

Render a Dioxus VirtualDOM to a string.


```rust
// Our app:
const App: Component = |cx|  rsx!(cx, div {"hello world!"});

// Build the virtualdom from our app
let mut vdom = VirtualDOM::new(App);

// This runs components, lifecycles, etc. without needing a physical dom. Some features (like noderef) won't work.
let _ = vdom.rebuild();

// Render the entire virtualdom from the root
let text = dioxus::ssr::render_vdom(&vdom);
assert_eq!(text, "<div>hello world!</div>")
```


## Usage in pre-rendering 

This crate is particularly useful in pre-generating pages server-side and then selectively loading dioxus client-side to pick up the reactive elements.

In fact, this crate supports hydration out of the box. However, it is extremely important that both the client and server will generate the exact same VirtualDOMs - the client picks up its VirtualDOM assuming that the pre-rendered page output is the same. To do this, you need to make sure that your VirtualDOM implementation is deterministic! This could involve either serializing our app state and sending it to the client, hydrating only parts of the page, or building tests to ensure what's rendered on the server is the same as the client.

With pre-rendering enabled, this crate will generate element nodes with Element IDs pre-associated. During hydration, the Dioxus-WebSys renderer will attach the Virtual nodes to these real nodes after a page query.

To enable pre-rendering, simply configure the `SsrConfig` with pre-rendering enabled.

```rust
let dom = VirtualDom::new(App);

let text = dioxus::ssr::render_vdom(App, |cfg| cfg.pre_render(true));
```

## Usage in server-side rendering

Dioxus SSR can also be to render on the server. Obviously, you can just render the VirtualDOM to a string and send that down.

```rust
let text = dioxus::ssr::render_vdom(&vdom);
assert_eq!(text, "<div>hello world!</div>")
```

The rest of the space - IE doing this more efficiently, caching the virtualdom, etc, will all need to be a custom implementation for now.

## Usage without a VirtualDom

Dioxus SSR needs an arena to allocate from - whether it be the VirtualDom or a dedicated Bump allocator. To render `rsx!` directly to a string, you'll want to create an `SsrRenderer` and call `render_lazy`.

```rust
let text = dioxus::ssr::SsrRenderer::new().render_lazy(rsx!{
    div { "hello world" }
});
assert_eq!(text, "<div>hello world!</div>")
```

This can be automated with the `render_lazy!` macro:

```rust
let text = render_lazy!(rsx!( div { "hello world" } ));
```

## Usage in static site generation

Dioxus SSR is a powerful tool to generate static sites. Using Dioxus for static site generation _is_ a bit overkill, however. The new documentation generation library, Doxie, is essentially Dioxus SSR on steroids designed for static site generation with client-side hydration.


Again, simply render the VirtualDOM to a string using `render_vdom` or any of the other render methods.
