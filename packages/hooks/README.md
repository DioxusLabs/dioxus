# Dioxus Hooks

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/dioxus-hooks.svg
[crates-url]: https://crates.io/crates/dioxus-hooks
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/dioxuslabs/dioxus/blob/master/LICENSE
[actions-badge]: https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg
[actions-url]: https://github.com/dioxuslabs/dioxus/actions?query=workflow%3ACI+branch%3Amaster
[discord-badge]: https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/XgGxMSkvUM

[Website](https://dioxuslabs.com) |
[Guides](https://dioxuslabs.com/learn/0.4/) |
[API Docs](https://docs.rs/dioxus-hooks/latest/dioxus_hooks) |
[Chat](https://discord.gg/XgGxMSkvUM)

## Overview

`dioxus-hooks` includes some basic useful hooks for dioxus:

- use_state
- use_ref
- use_future
- use_coroutine
- use_callback

Unlike React, none of these hooks are foundational since they all build off the primitive `cx.use_hook`.

This crate also provides a few helpful macros to get around some Rust lifetime management issues in async.

- `to_owned![]`
- `use_future()`
- `use_callback!()`

## Contributing

- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- Join the discord and ask questions!

## License

This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you shall be licensed as MIT without any additional
terms or conditions.
