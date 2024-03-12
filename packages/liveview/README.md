# Dioxus Liveview

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/dioxus-liveview.svg
[crates-url]: https://crates.io/crates/dioxus-liveview
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/dioxuslabs/dioxus/blob/master/LICENSE
[actions-badge]: https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg
[actions-url]: https://github.com/dioxuslabs/dioxus/actions?query=workflow%3ACI+branch%3Amaster
[discord-badge]: https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/XgGxMSkvUM

[Website](https://dioxuslabs.com) |
[Guides](https://dioxuslabs.com/learn/0.4/) |
[API Docs](https://docs.rs/dioxus-liveview/latest/dioxus_liveview) |
[Chat](https://discord.gg/XgGxMSkvUM)

## Overview

`dioxus-liveview` provides adapters for running the Dioxus VirtualDom over a WebSocket connection.

The current backend frameworks supported include:

- Axum

Dioxus-LiveView exports some primitives to wire up an app into an existing backend framework.

- A ThreadPool for spawning the `!Send` VirtualDom and interacting with it from WebSockets
- An adapter for transforming various socket types into the `LiveViewSocket` type
- The glue to load the interpreter into your app

## Contributing

- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- Join the discord and ask questions!

## License

This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you shall be licensed as MIT without any additional
terms or conditions.
