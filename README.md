# Rink: Like "Ink" but for Rust and Dioxus

Rink lets you build terminal user interfaces in Rust with [`Dioxus`](https://dioxuslabs.com/).


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

## Background

You can use Html-like semantics with stylesheets, inline styles, tree hierarchy, components, and more in your  [`text-based user interface (TUI)`](https://en.wikipedia.org/wiki/Text-based_user_interface) application.

Rink is basically a port of [Ink](https://github.com/vadimdemedes/ink) but for [`Rust`](https://www.rust-lang.org/) and [`Dioxus`](https://dioxuslabs.com/). Rink doesn't depend on Node.js or any other JavaScript runtime, so your binaries are portable and beautiful.

## Limitations

- **Subset of Html**
Terminals can only render a subset of HTML. We support as much as we can.
- **Particular frontend design**
Terminals and browsers are and look different. Therefore, the same design might not be the best to cover both renderers.


## Status

**WARNING: Rink is currently under construction!**

Rendering a VirtualDom works fine, but the ecosystem of hooks is not yet ready. Additionally, some bugs in the flexbox implementation might be quirky at times.

## Features

Rink features:
- [x] Flexbox based layout system
- [ ] CSS selectors
- [x] inline CSS support
- [ ] Built-in focusing system
- [ ] high-quality keyboard support
- [ ] Support for events, hooks, and callbacks
* [ ] Html tags<sup>1</sup>

<sup>1</sup> Currently, HTML tags don't translate into any meaning inside of rink. So an `input` won't really mean anything nor does it have any additional functionality.


