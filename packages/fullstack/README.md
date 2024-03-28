# Dioxus Fullstack

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/dioxus-fullstack.svg
[crates-url]: https://crates.io/crates/dioxus-fullstack
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/dioxuslabs/dioxus/blob/master/LICENSE
[actions-badge]: https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg
[actions-url]: https://github.com/dioxuslabs/dioxus/actions?query=workflow%3ACI+branch%3Amaster
[discord-badge]: https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/XgGxMSkvUM

[Website](https://dioxuslabs.com) |
[Guides](https://dioxuslabs.com/learn/0.5/) |
[API Docs](https://docs.rs/dioxus-fullstack/latest/dioxus_sever) |
[Chat](https://discord.gg/XgGxMSkvUM)

Fullstack utilities for the [`Dioxus`](https://dioxuslabs.com) framework.

# Features

- Integrates with the [Axum](https::/docs.rs/dioxus-fullstack/latest/dixous_server/axum_adapter/index.html) server framework with utilities for serving and rendering Dioxus applications.
- [Server functions](https::/docs.rs/dioxus-fullstack/latest/dixous_server/prelude/attr.server.html) allow you to call code on the server from the client as if it were a normal function.
- Instant RSX Hot reloading with [`dioxus-hot-reload`](https://crates.io/crates/dioxus-hot-reload).
- Passing root props from the server to the client.

# Example

Full stack Dioxus in under 30 lines of code

```rust
#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    launch(App);
}

#[component]
fn App() -> Element {
    let meaning = use_signal(|| None);

    rsx! {
        h1 { "Meaning of life: {meaning:?}" }
        button {
            onclick: move |_| async move {
                if let Ok(data) = get_meaning("life the universe and everything".into()).await {
                    meaning.set(data);
                }
            },
            "Run a server function"
        }
    }
}

#[server]
async fn get_meaning(of: String) -> Result<Option<u32>, ServerFnError> {
    Ok(of.contains("life").then(|| 42))
}
```

## Getting Started

To get started with full stack Dioxus, check out our [getting started guide](https://dioxuslabs.com/learn/0.5/getting_started), or the [full stack examples](https://github.com/DioxusLabs/dioxus/tree/master/packages/fullstack/examples).

## Contributing

- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- Join the discord and ask questions!

## License

This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you shall be licensed as MIT without any additional
terms or conditions.
