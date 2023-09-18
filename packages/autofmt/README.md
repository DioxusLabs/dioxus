# dioxus-autofmt

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/dioxus-autofmt.svg
[crates-url]: https://crates.io/crates/dioxus-autofmt
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/dioxuslabs/dioxus/blob/master/LICENSE
[actions-badge]: https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg
[actions-url]: https://github.com/dioxuslabs/dioxus/actions?query=workflow%3ACI+branch%3Amaster
[discord-badge]: https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/XgGxMSkvUM

[Website](https://dioxuslabs.com) |
[Guides](https://dioxuslabs.com/learn/0.4/) |
[API Docs](https://docs.rs/dioxus-autofmt/latest/dioxus_autofmt) |
[Chat](https://discord.gg/XgGxMSkvUM)

## Overview

`dioxus-autofmt` provides a pretty printer for the `rsx` syntax tree.

This is done manually with a via set of formatting rules. The output is not guaranteed to be stable between minor versions of the crate as we might tweak the output.

`dioxus-autofmt` provides an API to perform precision edits as well as just spit out a block of formatted RSX from any RSX syntax tree. This is used by the `rsx-rosetta` crate which can accept various input languages and output valid RSX.

## Contributing

- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- Join the discord and ask questions!

## License

This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you shall be licensed as MIT without any additional
terms or conditions.
