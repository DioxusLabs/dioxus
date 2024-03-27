# `dioxus-interpreter-js`

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/dioxus-interpreter-js.svg
[crates-url]: https://crates.io/crates/dioxus-interpreter-js
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/dioxuslabs/dioxus/blob/master/LICENSE
[actions-badge]: https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg
[actions-url]: https://github.com/dioxuslabs/dioxus/actions?query=workflow%3ACI+branch%3Amaster
[discord-badge]: https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/XgGxMSkvUM

[Website](https://dioxuslabs.com) |
[Guides](https://dioxuslabs.com/learn/0.5/) |
[API Docs](https://docs.rs/dioxus-interpreter-js/latest/dioxus_interpreter_js) |
[Chat](https://discord.gg/XgGxMSkvUM)

## Overview

`dioxus-interpreter-js` provides the high-performance JavaScript glue that interprets the stream of edits produced by the Dioxus VirtualDom and converts them into mutations on the actual web DOM.

This crate features bindings for the web and sledgehammer for increased performance.

## Architecture

We use TypeScript to write the bindings and a very simple build.rs along with bun to convert them to javascript, minify them, and glue them into the rest of the project.

Not every snippet of JS will be used, so we split out the snippets from the core interpreter.

In theory, we *could* use Rust in the browser to do everything these bindings are doing. In reality, we want to stick with JS to skip the need for a WASM build step when running the LiveView and WebView renderers. We also want to use JS to prevent diverging behavior of things like canceling events, uploading files, and collecting form inputs. These details are tough to ensure 1:1 compatibility when implementing them in two languages.

If you want to contribute to the bindings, you'll need to have the typescript compiler installed on your machine as well as bun:

https://bun.sh/docs/installation

## Contributing

- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- Join the discord and ask questions!

## License

This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you shall be licensed as MIT without any additional
terms or conditions.
