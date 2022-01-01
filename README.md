# Rink: Like "Ink" but for Rust and Dioxus

The fastest portable TUIs in the west 
 🔫🤠🔫
   🐎🔥🔥🔥

Rink lets you build terminal user interfaces in Rust with Dioxus. 

You can use html-esque semantics with stylesheets, inline styles, tree hierarchy, components, etc, but your Tui app is probably not going to work well or look good in the web. It still technically is a limited subset of HTML, so use at your own risk.

```rust

static App: FC<()> = |cx| {
    cx.render(rsx!{
        div { 
            width: "100%", 
            height: "10px",
            background_color: "red",
            justify_content: "center",
            align_items: "center",


            "Hello world!"
        }
    })
}
```

![demo app](examples/example.png)


Rink is basically a port of [Ink]() but for Rust and Dioxus. Rink doesn't depend on Node.js or any other JavaScript runtime, so your binaries are portable and beautiful.

## Features

Rink features:
- Flexbox based layout system
- CSS selectors
- inline css support
- Built-in focusing system
- high-quality keyboard support
- Support for events, hooks, and callbacks
- support for a very limited subset of HTML and CSS primitives

HTML elements supported:
- div, h1-h6, p, input, textarea, tables, nav

CSS Supported:
- Flex
