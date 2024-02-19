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
[Guides](https://dioxuslabs.com/learn/0.4/) |
[API Docs](https://docs.rs/dioxus-fullstack/latest/dioxus_sever) |
[Chat](https://discord.gg/XgGxMSkvUM)

Fullstack utilities for the [`Dioxus`](https://dioxuslabs.com) framework.

# Features

- Intigrations with the [Axum](https::/docs.rs/dioxus-fullstack/latest/dixous_server/axum_adapter/index.html), [Salvo](https::/docs.rs/dioxus-fullstack/latest/dixous_server/salvo_adapter/index.html), and [Warp](https::/docs.rs/dioxus-fullstack/latest/dixous_server/warp_adapter/index.html) server frameworks with utilities for serving and rendering Dioxus applications.
- [Server functions](https::/docs.rs/dioxus-fullstack/latest/dixous_server/prelude/attr.server.html) allow you to call code on the server from the client as if it were a normal function.
- Instant RSX Hot reloading with [`dioxus-hot-reload`](https://crates.io/crates/dioxus-hot-reload).
- Passing root props from the server to the client.

# Example

Full stack Dioxus in under 50 lines of code

```rust
#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_fullstack::prelude::*;

// On the web, run our client code
#[cfg(feature = "web")]
fn main() {
    dioxus_web::launch_with_props(
        app,
        get_root_props_from_document().unwrap_or_default(),
        dioxus_web::Config::new().hydrate(true),
    );
}

// On the server, run a simple warp server
#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    // Automatically handles server side rendering, hot reloading intigration, and hosting server functions
    warp::serve(serve_dioxus_application("", ServerConfig::new(app, ())))
        .run(([127, 0, 0, 1], 8080))
        .await;
}

fn app() -> Element {
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

To get started with full stack Dioxus, check out our [getting started guide](https://dioxuslabs.com/docs/nightly/guide/en/getting_started/ssr.html), or the [full stack examples](https://github.com/DioxusLabs/dioxus/tree/master/packages/fullstack/examples).

## Contributing

- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- Join the discord and ask questions!

## License

This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you shall be licensed as MIT without any additional
terms or conditions.
