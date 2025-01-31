# Dioxus Fullstack

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/dioxus-fullstack.svg
[crates-url]: https://crates.io/crates/dioxus-fullstack
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/dioxuslabs/dioxus/blob/main/LICENSE-MIT
[actions-badge]: https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg
[actions-url]: https://github.com/dioxuslabs/dioxus/actions?query=workflow%3ACI+branch%3Amaster
[discord-badge]: https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/XgGxMSkvUM

[Website](https://dioxuslabs.com) |
[Guides](https://dioxuslabs.com/learn/0.6/) |
[API Docs](https://docs.rs/dioxus-fullstack/latest/dioxus_fullstack/) |
[Chat](https://discord.gg/XgGxMSkvUM)

Fullstack utilities for the [`Dioxus`](https://dioxuslabs.com) framework.

# Features

- Integrates with the [Axum](./examples/axum-hello-world/src/main.rs) server framework with utilities for serving and rendering Dioxus applications.
- [Server functions](https://docs.rs/dioxus-fullstack/latest/dioxus_fullstack/prelude/attr.server.html) allow you to call code on the server from the client as if it were a normal function.
- Instant RSX Hot reloading with [`dioxus-hot-reload`](https://crates.io/crates/dioxus-hot-reload).
- Passing root props from the server to the client.

# Example

Full stack Dioxus in under 30 lines of code

```rust, no_run
#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut meaning = use_signal(|| None);

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

## Axum Integration

If you have an existing Axum router or you need more control over the server, you can use the [`DioxusRouterExt`](https://docs.rs/dioxus-fullstack/0.6.0-alpha.2/dioxus_fullstack/prelude/trait.DioxusRouterExt.html) trait to integrate with your existing Axum router.

First, make sure your `axum` dependency is optional and enabled by the server feature flag. Axum cannot be compiled to wasm, so if it is enabled by default, it will cause a compile error:

```toml
[dependencies]
dioxus = { version = "*", features = ["fullstack"] }
axum = { version = "0.7.0", optional = true }
tokio = { version = "1.0", features = ["full"], optional = true }
dioxus-cli-config = { version = "*", optional = true }

[features]
server = ["dioxus/server", "dep:axum", "dep:tokio", "dioxus-cli-config"]
web = ["dioxus/web"]
```

Then we can set up dioxus with the axum server:

```rust, no_run
#![allow(non_snake_case)]
use dioxus::prelude::*;

// The entry point for the server
#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    // Get the address the server should run on. If the CLI is running, the CLI proxies fullstack into the main address
    // and we use the generated address the CLI gives us
    let address = dioxus::cli_config::fullstack_address_or_localhost();

    // Set up the axum router
    let router = axum::Router::new()
        // You can add a dioxus application to the router with the `serve_dioxus_application` method
        // This will add a fallback route to the router that will serve your component and server functions
        .serve_dioxus_application(ServeConfigBuilder::default(), App);

    // Finally, we can launch the server
    let router = router.into_make_service();
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

// For any other platform, we just launch the app
#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut meaning = use_signal(|| None);

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

To get started with full stack Dioxus, check out our [getting started guide](https://dioxuslabs.com/learn/0.6/getting_started), or the [full stack examples](https://github.com/DioxusLabs/dioxus/tree/master/examples).

## Contributing

- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- Join the discord and ask questions!

## License

This project is licensed under the [MIT license].

[mit license]: https://github.com/dioxuslabs/dioxus/blob/main/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you shall be licensed as MIT without any additional
terms or conditions.
