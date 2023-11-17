# `dioxus-html-extension`: An extension to the Html (and SVG) Namespace for Dioxus

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/dioxus-html-extension.svg
[crates-url]: https://crates.io/crates/dioxus-html-extension
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/dioxuslabs/dioxus/blob/master/LICENSE
[actions-badge]: https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg
[actions-url]: https://github.com/dioxuslabs/dioxus/actions?query=workflow%3ACI+branch%3Amaster
[discord-badge]: https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/XgGxMSkvUM

[Website](https://dioxuslabs.com) |
[Guides](https://dioxuslabs.com/learn/0.4/) |
[API Docs](https://docs.rs/dioxus-html-extension/latest/dioxus_html) |
[Chat](https://discord.gg/XgGxMSkvUM)

## Overview

This crate provides an extension to the Html (and SVG) namespace for Dioxus.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
dioxus-html-extension = "0.4"
```

## Example

```rust
use dioxus::prelude::*;
use dioxus_html_extension::*;

fn app(cx: Scope) -> Element {
    render! {
        Image {
            src: image!("https://contrib.rocks/image?repo=dioxuslabs/dioxus&max=52&columns=13", { size: (686, 209), low_quality_preview: true }),
            width: "686px",
            height: "209px",
            alt: "A grid of all of the amazing contributors to Dioxus!",
        }
    }
}
```