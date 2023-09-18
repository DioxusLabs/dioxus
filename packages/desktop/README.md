# Dioxus Desktop (webview)

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/dioxus-desktop.svg
[crates-url]: https://crates.io/crates/dioxus-desktop
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/dioxuslabs/dioxus/blob/master/LICENSE
[actions-badge]: https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg
[actions-url]: https://github.com/dioxuslabs/dioxus/actions?query=workflow%3ACI+branch%3Amaster
[discord-badge]: https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/XgGxMSkvUM

[Website](https://dioxuslabs.com) |
[Guides](https://dioxuslabs.com/learn/0.4/) |
[API Docs](https://docs.rs/dioxus-desktop/latest/dioxus_desktop) |
[Chat](https://discord.gg/XgGxMSkvUM)

## Overview

`dioxus-desktop` provides a webview-based desktop renderer for the Dioxus VirtualDom.

This requires that webview is installed on the target system. WebView is installed by default on macOS and iOS devices, but might not come preinstalled on Windows or Linux devices. To fix these issues, follow the [instructions in the guide](guide-url).

[guide-url]: https://dioxuslabs.com/learn/0.4/getting_started/desktop

## Features

- Simple, one-line launch for desktop apps
- Dioxus VirtualDom running on a native thread
- Full HTML/CSS support via `wry` and `tao`
- Exposed `window` and `Proxy` types from tao for direct window manipulation
- Helpful hooks for accessing the window, WebView, and running javascript.

## Contributing

- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- Join the discord and ask questions!

## License

This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you shall be licensed as MIT without any additional
terms or conditions.
