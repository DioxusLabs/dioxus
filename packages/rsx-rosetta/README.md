# `rsx-rosetta`

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/rsx-rosetta.svg
[crates-url]: https://crates.io/crates/rsx-rosetta
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/dioxuslabs/dioxus/blob/master/LICENSE
[actions-badge]: https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg
[actions-url]: https://github.com/dioxuslabs/dioxus/actions?query=workflow%3ACI+branch%3Amaster
[discord-badge]: https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/XgGxMSkvUM

[Website](https://dioxuslabs.com) |
[Guides](https://dioxuslabs.com/learn/0.4/) |
[API Docs](https://docs.rs/rsx-rosetta/latest/rsx-rosetta) |
[Chat](https://discord.gg/XgGxMSkvUM)

## Overview

Dioxus sports its own templating language inspired by C#/Kotlin/RTMP, etc. It's pretty straightforward.

However, it's NOT HTML. This is done since HTML is verbose and you'd need a dedicated LSP or IDE integration to get a good DX in .rs files.

RSX is simple... It's similar enough to regular Rust code to trick most IDEs into automatically providing support for things like block selections, folding, highlighting, etc.

To accomodate the transition from HTML to RSX, you might need to translate some existing code.

This library provids a central AST that can accept a number of inputs:

- HTML
- Syn (todo)
- Akama (todo)
- Jinja (todo)

From there, you can convert directly to a string or into some other AST.

## Contributing

- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- Join the discord and ask questions!

## License

This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you, shall be licensed as MIT, without any additional
terms or conditions.
