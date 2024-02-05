<div align="center">
  <h1>Dioxus Server-Side Rendering (SSR)</h1>
  <p>
    <strong>Render Dioxus to valid html.</strong>
  </p>
</div>

## Resources

This crate is a part of the broader Dioxus ecosystem. For more resources about Dioxus, check out:

- [Getting Started](https://dioxuslabs.com/learn/0.4/getting_started)
- [Book](https://dioxuslabs.com/learn/0.4/)
- [Examples](https://github.com/DioxusLabs/example-projects)

## Overview

Dioxus SSR provides utilities to render Dioxus components to valid HTML. Once rendered, the HTML can be rehydrated client-side or served from your web server of choice.

```rust, ignore
let app: Component = |cx| rsx!(div {"hello world!"});

let mut vdom = VirtualDom::new(app);
let _ = vdom.rebuild();

let text = dioxus_ssr::render(&vdom);
assert_eq!(text, "<div>hello world!</div>")
```

## Basic Usage

The simplest example is to simply render some `rsx!` nodes to HTML. This can be done with the [`render_element`] API.

```rust, ignore
let content = dioxus_ssr::render_element(rsx!{
    div {
        (0..5).map(|i| rsx!(
            "Number: {i}"
        ))
    }
});
```

## Rendering a VirtualDom

```rust, ignore
let mut vdom = VirtualDom::new(app);
let _ = vdom.rebuild();

let content = dioxus_ssr::render(&vdom);
```

## Usage in pre-rendering

This crate is particularly useful in pre-generating pages server-side and then selectively loading Dioxus client-side to pick up the reactive elements.

This crate supports hydration out of the box. However, both the client and server must generate the _exact_ same VirtualDOMs - the client picks up its VirtualDOM assuming that the pre-rendered page output is the same. To do this, you need to make sure that your VirtualDOM implementation is deterministic! This could involve either serializing our app state and sending it to the client, hydrating only parts of the page, or building tests to ensure what's rendered on the server is the same as the client.

With pre-rendering enabled, this crate will generate element nodes with Element IDs pre-associated. During hydration, the Dioxus-WebSys renderer will attach the Virtual nodes to these real nodes after a page query.

To enable pre-rendering, simply set the pre-rendering flag to true.

```rust, ignore
let mut vdom = VirtualDom::new(App);

let _ = vdom.rebuild();

let mut renderer = dioxus_ssr::Renderer::new();
renderer.pre_render = true;

let text = renderer.render(&vdom);
```

## Usage in server-side rendering

Dioxus SSR can also be used to render on the server. You can just render the VirtualDOM to a string and send that to the client.

```rust, ignore
let text = dioxus_ssr::render(&vdom);
assert_eq!(text, "<div>hello world!</div>")
```

The rest of the space - IE doing this more efficiently, caching the VirtualDom, etc, will all need to be a custom implementation for now.

## Usage without a VirtualDom

Dioxus SSR needs an arena to allocate from - whether it be the VirtualDom or a dedicated Bump allocator. To render `rsx!` directly to a string, you'll want to create a `Renderer` and call `render_element`.

```rust, ignore
let text = dioxus_ssr::Renderer::new().render_element(rsx!{
    div { "hello world" }
});
assert_eq!(text, "<div>hello world!</div>")
```

This can be automated with the `render_element!` macro:

```rust, ignore
let text = render_element!(rsx!( div { "hello world" } ));
```

## Usage in static site generation

Dioxus SSR is a powerful tool to generate static sites. Using Dioxus for static site generation _is_ a bit overkill, however. The new documentation generation library, Doxie, is essentially Dioxus SSR on steroids designed for static site generation with client-side hydration.

Again, simply render the VirtualDOM to a string using `render` or any of the other render methods.
