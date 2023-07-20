> This guide assumes you read the [Web](web.md) guide and installed the [Dioxus-cli](https://github.com/DioxusLabs/cli)

# Getting Started

## Setup

For this guide, we're going to show how to use Dioxus with [Axum](https://docs.rs/axum/latest/axum/), but `dioxus-fullstack` also integrates with the [Warp](https://docs.rs/warp/latest/warp/) and [Salvo](https://docs.rs/salvo/latest/salvo/) web frameworks.

Make sure you have Rust and Cargo installed, and then create a new project:

```shell
cargo new --bin demo
cd demo
```

Add `dioxus` and `dioxus-fullstack` as dependencies:

```shell
cargo add dioxus
cargo add dioxus-fullstack --features axum, ssr
```

Next, add all the Axum dependencies. This will be different if you're using a different Web Framework

```shell
cargo add tokio --features full
cargo add axum
```

Your dependencies should look roughly like this:

```toml
[dependencies]
axum = "*"
dioxus = { version = "*" }
dioxus-fullstack = { version = "*", features = ["axum", "ssr"] }
tokio = { version = "*", features = ["full"] }
```

Now, set up your Axum app to serve the Dioxus app.

```rust, no_run
{{#include ../../../examples/server_basic.rs}}
```

Now, run your app with `cargo run` and open `http://localhost:8080` in your browser. You should see a server-side rendered page with a counter.

## Hydration

Right now, the page is static. We can't interact with the buttons. To fix this, we can hydrate the page with `dioxus-web`.

First, modify your `Cargo.toml` to include two features, one for the server called `ssr`, and one for the client called `web`.

```toml
[dependencies]
# Common dependancies
dioxus = { version = "*" }
dioxus-fullstack = { version = "*" }

# Web dependancies
dioxus-web = { version = "*", features=["hydrate"], optional = true }

# Server dependancies
axum = { version = "0.6.12", optional = true }
tokio = { version = "1.27.0", features = ["full"], optional = true }

[features]
default = []
ssr = ["axum", "tokio", "dioxus-fullstack/axum"]
web = ["dioxus-web"]
```

Next, we need to modify our `main.rs` to use either hydrate on the client or render on the server depending on the active features.

```rust, no_run
{{#include ../../../examples/hydration.rs}}
```

Now, build your client-side bundle with `dx build --features web` and run your server with `cargo run --features ssr`. You should see the same page as before, but now you can interact with the buttons!

## Sycronizing props between the server and client

Let's make the initial count of the counter dynamic based on the current page.

### Modifying the server

To do this, we must remove the serve_dioxus_application and replace it with a custom implementation of its four key functions:

- Serve static WASM and JS files with serve_static_assets
- Register server functions with register_server_fns (more information on server functions later)
- Connect to the hot reload server with connect_hot_reload
- A custom route that uses SSRState to server-side render the application

### Modifying the client

The only thing we need to change on the client is the props. `dioxus-fullstack` will automatically serialize the props it uses to server render the app and send them to the client. In the client section of `main.rs`, we need to add `get_root_props_from_document` to deserialize the props before we hydrate the app.

```rust, no_run
{{#include ../../../examples/hydration_props.rs}}
```

Now, build your client-side bundle with `dx build --features web` and run your server with `cargo run --features ssr`. Navigate to `http://localhost:8080/1` and you should see the counter start at 1. Navigate to `http://localhost:8080/2` and you should see the counter start at 2.
