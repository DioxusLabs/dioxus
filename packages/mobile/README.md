# Dioxus Mobile

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/dioxus-mobile.svg
[crates-url]: https://crates.io/crates/dioxus-mobile
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/dioxuslabs/dioxus/blob/master/LICENSE
[actions-badge]: https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg
[actions-url]: https://github.com/dioxuslabs/dioxus/actions?query=workflow%3ACI+branch%3Amaster
[discord-badge]: https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/XgGxMSkvUM

[Website](https://dioxuslabs.com) |
[Guides](https://dioxuslabs.com/learn/0.4/) |
[API Docs](https://docs.rs/dioxus-mobile/latest/dioxus_mobile) |
[Chat](https://discord.gg/XgGxMSkvUM)

## Overview

`dioxus-mobile` is a re-export of `dioxus-desktop` with some minor tweaks and documentation changes. As this crate evolves, it will provide some more unique features to mobile, but for now, it's very similar to the desktop crate.

Dioxus Mobile supports both iOS and Android. However, Android support is still quite experimental and requires a lot of configuration. A good area to contribute here would be to improve the CLI tool to include bundling and mobile configuration.

## Getting Set up

Getting set up with mobile can but quite challenging. The tooling here isn't great (yet) and might take some hacking around to get things working. macOS M1 is broadly unexplored and might not work for you.

We're going to be using `cargo-mobile` to build for mobile. First, install it:

```shell
$ cargo install --git https://github.com/BrainiumLLC/cargo-mobile
```

And then initialize your app for the right platform. Use the `winit` template for now. Right now, there's no "Dioxus" template in cargo-mobile.

```shell
$ cargo mobile init
```

We're going to completely clear out the `dependencies` it generates for us, swapping out `winit` with `dioxus-mobile`.

```toml

[package]
name = "dioxus-ios-demo"
version = "0.1.0"
authors = ["Jonathan Kelley <jkelleyrtp@gmail.com>"]
edition = "2021"


# leave the `lib` declaration
[lib]
crate-type = ["staticlib", "cdylib", "rlib"]


# leave the binary it generates for us
[[bin]]
name = "dioxus-ios-demo-desktop"
path = "gen/bin/desktop.rs"

# clear all the dependencies
[dependencies]
mobile-entry-point = "0.1.0"
dioxus = { version = "*", features = ["mobile"] }
simple_logger = "*"
```

Edit your `lib.rs`:

```rust
// main.rs
use dioxus::prelude::*;

fn main() {
    dioxus_mobile::launch(app);
}

fn app() -> Element {
    rsx!{
        div {
            "hello world!"
        }
    })
}
```

To configure the web view, menubar, and other important desktop-specific features, checkout out some of the launch configurations in the [API reference](https://docs.rs/dioxus-mobile/).

## Future Steps

Make sure to read the [Dioxus Guide](https://dioxuslabs.com/learn/0.4/) if you already haven't!

## Contributing

- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- Join the discord and ask questions!

## License

This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you shall be licensed as MIT without any additional
terms or conditions.
