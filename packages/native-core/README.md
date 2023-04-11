# Dioxus Native Core

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/dioxus-native-core.svg
[crates-url]: https://crates.io/crates/dioxus-native-core

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/dioxuslabs/dioxus/blob/master/LICENSE

[actions-badge]: https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg
[actions-url]: https://github.com/dioxuslabs/dioxus/actions?query=workflow%3ACI+branch%3Amaster

[discord-badge]: https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/XgGxMSkvUM

[Website](https://dioxuslabs.com) |
[Guides](https://dioxuslabs.com/guide/) |
[API Docs](https://docs.rs/dioxus-native-core/latest/dioxus_native_core) |
[Chat](https://discord.gg/XgGxMSkvUM)


## Overview

`dioxus-native-core` provides several helpful utilities for lazily resolving computed values of the Dioxus VirtualDom to be used in conjunction with a native rendering engine.

The main "value-add" of this crate over implementing your native tree is that this tree is incrementally recomputed using the Dioxus VirtualDom's edit stream. Only parts of the tree that rely on each other will be redrawn - all else will be ignored.


## Contributing

- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- Join the discord and ask questions!

## License
This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you shall be licensed as MIT without any additional
terms or conditions.
