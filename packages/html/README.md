# `dioxus-html`: Html (and SVG) Namespace for Dioxus

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/dioxus-html.svg
[crates-url]: https://crates.io/crates/dioxus-html
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/dioxuslabs/dioxus/blob/main/LICENSE-MIT
[actions-badge]: https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg
[actions-url]: https://github.com/dioxuslabs/dioxus/actions?query=workflow%3ACI+branch%3Amaster
[discord-badge]: https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/XgGxMSkvUM

[Website](https://dioxuslabs.com) |
[Guides](https://dioxuslabs.com/learn/0.7/) |
[API Docs](https://docs.rs/dioxus-html/latest/dioxus_html) |
[Chat](https://discord.gg/XgGxMSkvUM)

## Overview

The Dioxus `rsx!` macro can accept any compile-time correct namespace on top of NodeFactory. This crate provides the HTML (and SVG) namespaces which get imported in the Dioxus prelude.

However, this abstraction enables you to add any namespace of elements, provided they're in scope when rsx! is called. For an example, a UI that is designed for Augmented Reality might use different primitives than HTML:

```rust, ignore
use ar_namespace::*;

rsx! {
    magic_div {
        magic_header {}
        magic_paragraph {
            on_magic_click: move |event| {
                //
            }
        }
    }
}
```

This is currently a not-very-explored part of Dioxus. However, the namespacing system does make it possible to provide syntax highlighting, documentation, "go to definition" and compile-time correctness, so it's worth having it abstracted.

## How it works:

Elements for dioxus must implement the (simple) DioxusElement trait to be used in the rsx! macro.

```rust, ignore
struct div;
impl DioxusElement for div {
    const TAG_NAME: &'static str = "div";
    const NAME_SPACE: Option<&'static str> = None;
}
```

All elements should be defined as a zero-sized-struct (also known as unit struct). These structs are zero-cost and just provide the type-level trickery to Rust for compile-time correct templates.

Attributes would then be implemented as constants on these unit structs.

The HTML namespace is defined mostly with macros. However, the expanded form would look something like this:

```rust, ignore
struct base;
impl DioxusElement for base {
    const TAG_NAME: &'static str = "base";
    const NAME_SPACE: Option<&'static str> = None;
}
impl base {
    const href: (&'static str, Option<'static str>, bool) = ("href", None, false);
    const target: (&'static str, Option<'static str>, bool) = ("target", None, false);
}
```

Because attributes are defined as methods on the unit struct, they guard the attribute creation behind a compile-time correct interface.

## How to extend it:

Whenever the rsx! macro is called, it relies on the HTML element constructors and extension traits that are in scope. When you enable the `html` feature in dioxus, the prelude imports the built-in HTML namespace for you. You can extend this with your own custom elements by adding a constructor for the tag and extension traits for any typed attributes you want to support.

```rust, ignore
use dioxus::{
    core::view::{El, TagName, el},
    prelude::*,
};

pub struct AnalyticsPanel;

impl TagName for AnalyticsPanel {
    const NAME: &'static str = "analytics-panel";
}

pub const fn analytics_panel() -> El<AnalyticsPanel, (), ()> {
    el::<AnalyticsPanel>()
}

rsx! {
    analytics_panel {
        "Rendered as <analytics-panel>"
    }
}
```

See [`examples/09-reference/custom_element.rs`](../../examples/09-reference/custom_element.rs) for a complete example with a typed custom attribute.

## Contributing

- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- Join the discord and ask questions!

## License

This project is licensed under the [MIT license].

[mit license]: https://github.com/dioxuslabs/dioxus/blob/main/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you shall be licensed as MIT without any additional
terms or conditions.
