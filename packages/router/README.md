# Dioxus Native Core

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/dioxus-router.svg
[crates-url]: https://crates.io/crates/dioxus-router

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/dioxuslabs/dioxus/blob/master/LICENSE

[actions-badge]: https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg
[actions-url]: https://github.com/dioxuslabs/dioxus/actions?query=workflow%3ACI+branch%3Amaster

[discord-badge]: https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/XgGxMSkvUM

[Website](https://dioxuslabs.com) |
[Guides](https://dioxuslabs.com/docs/0.3/guide/en/) |
[API Docs](https://docs.rs/dioxus-router/latest/dioxus_router) |
[Chat](https://discord.gg/XgGxMSkvUM)


## Overview

Dioxus Router is a first-party Router for all your Dioxus Apps. It provides a React-Router-style interface using somewhat loose typing rules.

```rust, ignore
fn app() {
    cx.render(rsx! {
        Router {
            Route { to: "/", Component {} },
            Route { to: "/blog", Blog {} },
            Route { to: "/blog/:id", BlogPost {} },
        }
    })
}
```

You need to enable the right features for the platform you're targeting since these are not determined automatically!

## Contributing

- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- Join the discord and ask questions!

## License
This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you shall be licensed as MIT without any additional
terms or conditions.

