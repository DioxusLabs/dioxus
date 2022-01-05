<div align="center">
  <h1>Dioxus Server-Side Rendering (SSR)</h1>
  <p>
    <strong>Render Dioxus to valid html.</strong>
  </p>
</div>

## Resources
This crate is a part of the broader Dioxus ecosystem. For more resources about Dioxus, check out:

- [Getting Started](https://dioxuslabs.com/getting-started)
- [Book](https://dioxuslabs.com/book)
- [Reference](https://dioxuslabs.com/reference)
- [Community Examples](https://github.com/DioxusLabs/community-examples)

## Overview

Dioxus SSR provides utilities to render Dioxus components to valid HTML. Once rendered, the HTML can be rehydrated client side or served from your web-server of choice.

```rust, ignore
let app: Component = |cx| cx.render(rsx!(div {"hello world!"}));

let mut vdom = VirtualDom::new(app);
let _ = vdom.rebuild();

let text = dioxus::ssr::render_vdom(&vdom);
assert_eq!(text, "<div>hello world!</div>")
```


## Basic Usage

The simplest example is to simply render some `rsx!` nodes to html. This can be done with the [`render_lazy`] api.

```rust, ignore
let content = dioxus::ssr::render(rsx!{
    div {
        (0..5).map(|i| rsx!(
            "Number: {i}"
        ))
    }
});
```

## Rendering a VirtualDom

```rust, ignore
let mut dom = VirtualDom::new(app);
let _ = dom.rebuild();

let content = dioxus::ssr::render_vdom(&dom);
```

## Configuring output
It's possible to configure the output of the generated HTML. 

```rust, ignore
let content = dioxus::ssr::render_vdom(&dom, |config| config.pretty(true).prerender(true));
```

## Usage as a writer

We provide the basic `SsrFormatter` object that implements `Display`, so you can integrate SSR into an existing string, or write directly to a file.

```rust, ignore
use std::fmt::{Error, Write};

let mut buf = String::new();

let dom = VirtualDom::new(app);
let _ = dom.rebuild();

let args = dioxus::ssr::formatter(dom, |config| config);
buf.write_fmt!(format_args!("{}", args));
```

## Configuration







## Usage in pre-rendering 

This crate is particularly useful in pre-generating pages server-side and then selectively loading dioxus client-side to pick up the reactive elements.

In fact, this crate supports hydration out of the box. However, it is extremely important that both the client and server will generate the exact same VirtualDOMs - the client picks up its VirtualDOM assuming that the pre-rendered page output is the same. To do this, you need to make sure that your VirtualDOM implementation is deterministic! This could involve either serializing our app state and sending it to the client, hydrating only parts of the page, or building tests to ensure what's rendered on the server is the same as the client.

With pre-rendering enabled, this crate will generate element nodes with Element IDs pre-associated. During hydration, the Dioxus-WebSys renderer will attach the Virtual nodes to these real nodes after a page query.

To enable pre-rendering, simply configure the `SsrConfig` with pre-rendering enabled.

```rust, ignore
let dom = VirtualDom::new(App);

let text = dioxus::ssr::render_vdom(App, |cfg| cfg.pre_render(true));
```

## Usage in server-side rendering

Dioxus SSR can also be to render on the server. Obviously, you can just render the VirtualDOM to a string and send that down.

```rust, ignore
let text = dioxus::ssr::render_vdom(&vdom);
assert_eq!(text, "<div>hello world!</div>")
```

The rest of the space - IE doing this more efficiently, caching the virtualdom, etc, will all need to be a custom implementation for now.

## Usage without a VirtualDom

Dioxus SSR needs an arena to allocate from - whether it be the VirtualDom or a dedicated Bump allocator. To render `rsx!` directly to a string, you'll want to create an `SsrRenderer` and call `render_lazy`.

```rust, ignore
let text = dioxus::ssr::SsrRenderer::new().render_lazy(rsx!{
    div { "hello world" }
});
assert_eq!(text, "<div>hello world!</div>")
```

This can be automated with the `render_lazy!` macro:

```rust, ignore
let text = render_lazy!(rsx!( div { "hello world" } ));
```

## Usage in static site generation

Dioxus SSR is a powerful tool to generate static sites. Using Dioxus for static site generation _is_ a bit overkill, however. The new documentation generation library, Doxie, is essentially Dioxus SSR on steroids designed for static site generation with client-side hydration.


Again, simply render the VirtualDOM to a string using `render_vdom` or any of the other render methods.
